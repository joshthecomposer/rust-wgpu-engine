use glam::{Quat, Vec3};
use nalgebra::Vector3;
use rapier3d::prelude::*;

use crate::util::constants::{GROUP_PLAYER, GROUP_TERRAIN};
use crate::{
    entity_manager::EntityManager,
    enums_types::{PhysicsHandle, Transform},
    util::data_structure::HashMapGetPair,
};

pub struct PhysicsState {
    pub pipeline: PhysicsPipeline,
    pub gravity: Vector3<f32>,
    pub integration_parameters: IntegrationParameters,
    pub island_manager: IslandManager,
    pub broad_phase: BroadPhaseMultiSap,
    pub narrow_phase: NarrowPhase,
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: Option<QueryPipeline>,
    pub physics_hooks: (),
    pub event_handler: (),

    // accumulator stuff
    pub accumulator: f32,
    pub fixed_dt: f32,
}

impl PhysicsState {
    pub fn new() -> Self {
        Self {
            pipeline: PhysicsPipeline::new(),
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            integration_parameters: IntegrationParameters::default(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhaseMultiSap::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            gravity: vector![0.0, -9.81, 0.0],
            ccd_solver: CCDSolver::new(),
            query_pipeline: Some(QueryPipeline::new()),
            physics_hooks: (),
            event_handler: (),

            accumulator: 0.0,
            fixed_dt: 1.0 / 60.0,
        }
    }

    pub fn step(&mut self) {
        self.pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            self.query_pipeline.as_mut(),
            &self.physics_hooks,
            &self.event_handler,
        );
    }
}

pub fn apply_delta_v(rb: &mut RigidBody, dir: glam::Vec3, dv: f32) {
    let J = dir.normalize() * (rb.mass() * dv);
    rb.apply_impulse(J.into(), true);
}

pub fn jump_to_height(rb: &mut RigidBody, h: f32, gravity: f32) {
    let v0 = (2.0 * gravity.abs() * h).sqrt();
    let J = glam::vec3(0.0, rb.mass() * v0, 0.0);
    rb.apply_impulse(J.into(), true);
}

pub fn sync_transforms_from_physics(em: &mut EntityManager, ps: &PhysicsState) {
    let mut updates: Vec<(usize, glam::Vec3, glam::Quat)> =
        Vec::with_capacity(em.physics_handles.len());

    for ph in em.physics_handles.iter() {
        let id = ph.key();
        let PhysicsHandle { rigid_body, .. } = *ph.value();

        if let Some(rb) = ps.rigid_body_set.get(rigid_body) {
            let iso = rb.position();
            let pos = glam::Vec3::from_slice(iso.translation.vector.as_slice());
            let rot = {
                let c = iso.rotation.coords;
                glam::Quat::from_xyzw(c.x, c.y, c.z, c.w)
            };
            updates.push((id, pos, rot));
        }
    }

    // Apply to ECS transforms
    for (id, pos, rot) in updates {
        if let Some(t) = em.transforms.get_mut(id) {
            t.position = pos;
            t.rotation = rot;
            // keep existing t.scale as-is
        } else {
            panic!("SOmething didn't have a transform");
            em.transforms.insert(
                id,
                Transform {
                    position: pos,
                    rotation: rot,
                    scale: glam::Vec3::splat(1.0), // or preserve a known scale (e.g., Vec3::ONE)
                },
            );
        }
    }
}

pub fn push_weapon_kinematics_from_bones(em: &mut EntityManager, ps: &mut PhysicsState) {
    for wid in em.get_active_weapon_ids() {
        let parent = *em.owners.get(wid).unwrap();
        let animator = em.animators.get(parent).unwrap();
        let cur = animator.current_animation.clone();
        let next = animator.next_animation.clone();
        let blend = animator.blend_factor;

        let pt = em.transforms.get(parent).unwrap();
        let pm = glam::Mat4::from_scale_rotation_translation(pt.scale, pt.rotation, pt.position);
        let skel = em.skellingtons.get(parent).unwrap();
        let rh = em.item_bones.get(parent).unwrap().rh_name.clone();

        let bone_m = if blend > 0.0 && cur != next {
            let (a1, a2) = animator.animations.get_pair(&cur, &next).unwrap();
            a1.get_raw_global_bone_transform_by_name_blended(&rh, skel, pm, a2, blend)
        } else {
            animator
                .animations
                .get(&cur)
                .unwrap()
                .get_raw_global_bone_transform_by_name(&rh, skel, pm)
        };

        if let (Some(m), Some(ph)) = (bone_m, em.physics_handles.get(wid)) {
            //let (_s, rot, pos) = m.to_scale_rotation_translation();

            let corr = em.local_corrections.get(wid).cloned().unwrap_or(Transform {
                position: glam::Vec3::ZERO,
                rotation: glam::Quat::IDENTITY,
                scale: glam::Vec3::ONE,
            });

            let corr_m = glam::Mat4::from_scale_rotation_translation(
                corr.scale,
                corr.rotation,
                corr.position,
            );

            // Apply correction in bone space
            // (boneWorld * correctionLocal) -> final weapon world
            let final_m = m * corr_m;

            let (_, rot, pos) = final_m.to_scale_rotation_translation();

            if let Some(rb) = ps.rigid_body_set.get_mut(ph.rigid_body) {
                if rb.is_kinematic() {
                    let iso = rapier3d::na::Isometry3::from_parts(
                        rapier3d::na::Translation3::new(pos.x, pos.y, pos.z),
                        rapier3d::na::UnitQuaternion::from_quaternion(
                            rapier3d::na::Quaternion::new(rot.w, rot.x, rot.y, rot.z),
                        ),
                    );
                    rb.set_next_kinematic_position(iso);
                }
            }
        }
    }
}

pub fn push_static_kinematics(em: &EntityManager, ps: &mut PhysicsState) {
    for id in em.selected.iter() {
        if let Some(ph) = em.physics_handles.get(*id) {
            let rb = ps.rigid_body_set.get_mut(ph.rigid_body).unwrap();

            rb.wake_up(true);

            let gt = em.transforms.get(*id).unwrap();

            let iso = rapier3d::na::Isometry::from_parts(
                rapier3d::na::Translation3::new(gt.position.x, gt.position.y, gt.position.z),
                rapier3d::na::UnitQuaternion::from_quaternion(gt.rotation.into()),
            );

            rb.set_next_kinematic_position(iso);
        }
    }
}

pub fn sync_collider_transforms_with_physics(em: &mut EntityManager, ps: &mut PhysicsState) {
    for entry in em.physics_handles.iter() {
        let id = entry.key();
        let ph = entry.value();

        let collider = match ps.collider_set.get(ph.collider) {
            Some(c) => c,
            None => continue,
        };

        // Rapier gives collider *center*
        let iso: &rapier3d::na::Isometry3<f32> = collider.position();
        let t = iso.translation.vector;
        let r = iso.rotation;

        let center = Vec3::new(t.x, t.y, t.z);
        let rot = Quat::from_xyzw(r.i, r.j, r.k, r.w);

        let shape = collider.shape();

        let mut gizmo_pos = center;
        let mut gizmo_scale = Vec3::ONE;

        if let Some(cuboid) = shape.as_cuboid() {
            let he = cuboid.half_extents;

            let local_down = Vec3::new(0.0, he.y, 0.0);
            gizmo_pos = center - rot * local_down;
        }

        if let Some(cap) = shape.as_capsule() {
            let a = cap.segment.a;
            let b = cap.segment.b;
            let radius = cap.radius;

            let half_seg = 0.5 * (b - a).norm();

            let local_down = Vec3::new(0.0, half_seg + radius, 0.0);
            gizmo_pos = center - rot * local_down;
        }

        em.collider_transforms.insert(
            id,
            Transform {
                position: gizmo_pos,
                rotation: rot,
                scale: Vec3::ONE,
            },
        );
    }
}

pub fn grounding_solver(em: &mut EntityManager, ps: &PhysicsState) {
    let ids = vec![
        em.get_ids_for_type("TrashGuy"),
        em.get_ids_for_type("MooseMan"),
    ]
    .concat();

    for id in ids.iter() {
        let ph = em.physics_handles.get(*id).unwrap();
        let ch = ph.collider;
        let rb_handle = ph.rigid_body;

        let trans = em.transforms.get(*id).unwrap();
        let colliders = &ps.collider_set;
        let bodies = &ps.rigid_body_set;
        let query = ps.query_pipeline.as_ref().unwrap();
        //let r = collider.radius;
        let gs = em.grounded_states.get_mut(*id).unwrap();
        let position = trans.position;

        let ray = Ray::new(
            point![position.x, position.y + 0.02, position.z],
            vector![0.0, -1.0, 0.0],
        );

        let filter = QueryFilter::default()
            .groups(InteractionGroups::new(
                GROUP_PLAYER.into(),
                GROUP_TERRAIN.into(),
            ))
            .exclude_rigid_body(rb_handle)
            .exclude_sensors()
            .into();

        let dist = match gs.is_grounded {
            true => gs.ray_length_grounded,
            false => gs.ray_length_airborn,
        };

        let prev = gs.is_grounded;
        let result = query.cast_ray(bodies, colliders, &ray, dist, true, filter);

        gs.is_grounded = result.is_some();
        gs.just_landed = !prev && gs.is_grounded;
        gs.just_left = prev && !gs.is_grounded;
    }
}
