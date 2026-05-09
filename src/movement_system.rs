use gl::PixelStoref;
use glam::{vec3, Quat, Vec3};
use winit::keyboard::KeyCode;

use crate::{
    camera::CamMoveBasis,
    command_buffer::{CommandBuffer, ImpulseKind, LocoSpace},
    entity_manager::{glam_to_nalgebra_quat, EntityManager},
    enums_types::{AnimationType, Rotator},
    input::InputState,
    physics::PhysicsState,
};

pub fn update(
    em: &mut EntityManager,
    cam_basis: &CamMoveBasis,
    cmds: &mut CommandBuffer,
    phys: &PhysicsState,
    dt: f32,
) {
    let loco_cmds = std::mem::take(&mut cmds.loco);

    for lc in loco_cmds {
        let e = lc.target;

        let Some(speed) = em.base_speeds.get(e).copied() else {
            continue;
        };

        let current_vel = current_physics_velocity(em, phys, e).unwrap_or(Vec3::ZERO);
        let intent = Vec3::new(lc.intent.x, 0.0, lc.intent.z);
        let intent_dir = match lc.space {
            LocoSpace::World => normalize_flat_dir(intent),
            LocoSpace::Camera => resolve_world_intent_dir_camera(cam_basis, intent),
        };

        if lc.allow_trans {
            // During knockback, ignore locomotion steering so hit reaction can play out.
            if !em.knockbacks.contains(e) {
                let linvel = resolve_translation(intent_dir, speed, current_vel);
                cmds.set_linvel(e, ImpulseKind::Locomotion, linvel);
            }
        }

        if lc.allow_rot {
            let Some((yaw, desired_rot)) = desired_rotation_from_dir(intent_dir) else {
                continue;
            };

            em.yaws.insert(e, yaw);

            let Some(rotator) = em.rotators.get_mut(e) else {
                continue;
            };

            apply_rotation(e, rotator, desired_rot, cmds, dt);
        }
    }
}

fn current_physics_velocity(em: &EntityManager, phys: &PhysicsState, e: usize) -> Option<Vec3> {
    let ph = em.physics_handles.get(e)?;
    let rb = phys.rigid_body_set.get(ph.rigid_body)?;
    let v = rb.linvel();

    Some(Vec3::new(v.x, v.y, v.z))
}

fn resolve_world_intent_dir_camera(cam_basis: &CamMoveBasis, intent: Vec3) -> Option<Vec3> {
    normalize_flat_dir(cam_basis.right_flat * intent.x + cam_basis.fwd_flat * intent.z)
}

fn resolve_translation(intent_dir: Option<Vec3>, speed: f32, current_vel: Vec3) -> Vec3 {
    match intent_dir {
        Some(dir) => Vec3::new(dir.x * speed, current_vel.y, dir.z * speed),
        None => Vec3::new(0.0, current_vel.y, 0.0),
    }
}

fn desired_rotation_from_dir(dir: Option<Vec3>) -> Option<(f32, Quat)> {
    let dir = dir.and_then(normalize_flat_dir)?;
    let yaw = f32::atan2(dir.x, dir.z);

    Some((yaw, Quat::from_rotation_y(yaw)))
}

fn apply_rotation(
    e: usize,
    rotator: &mut Rotator,
    desired_rot: Quat,
    cmds: &mut CommandBuffer,
    dt: f32,
) {
    if rotator.blend_factor == 0.0 && !quat_approx_eq(rotator.cur_rot, desired_rot) {
        rotator.next_rot = desired_rot;
    }

    if !quat_approx_eq(rotator.next_rot, rotator.cur_rot) {
        rotator.blend_factor += dt / rotator.blend_time.max(0.0001);

        if rotator.blend_factor >= 1.0 {
            rotator.blend_factor = 0.0;
            rotator.cur_rot = rotator.next_rot;
        }
    }

    let smoothed = rotator
        .cur_rot
        .slerp(rotator.next_rot, rotator.blend_factor);

    cmds.set_rot(e, ImpulseKind::Locomotion, smoothed);
}

fn normalize_flat_dir(v: Vec3) -> Option<Vec3> {
    let flat = Vec3::new(v.x, 0.0, v.z);

    if flat.length_squared() > 0.000001 {
        Some(flat.normalize())
    } else {
        None
    }
}

fn quat_approx_eq(a: Quat, b: Quat) -> bool {
    a.dot(b).abs() > 0.9999
}
