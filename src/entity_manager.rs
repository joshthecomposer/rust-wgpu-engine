#![allow(clippy::too_many_arguments)]

use core::f32;
use std::collections::HashSet;

use glam::{Mat4, Quat, Vec3};
use nalgebra::{Point3, UnitQuaternion};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rapier3d::{parry::{shape::Capsule, utils::hashmap::HashMap}, prelude::*};

use crate::{animation::animation::{import_bone_data, import_model_data, Animation, Animator, Bone, Model}, config::{entity_config::{AnimationPropHelper, EntityConfig, EntityTypeHelper, ItemBones}, world_data::{EntityInstance, WorldData}}, debug::gizmos::{Cuboid, Cylinder, Pill}, enums_types::{ActiveItem, AttackState, EntityType, Faction, FrameActivation, HitboxType, Inventory, Knockback, Parent, PhysicsHandle, PlayerController, PlayerState, Rotator, SimState, SimStateController, Transform, VisualEffect}, physics::PhysicsState, sound::sound_manager::{ContinuousSound, OneShot, SoundManager}, sparse_set::SparseSet};

pub struct EntityManager {
    pub next_entity_id: usize,
    pub transforms: SparseSet<Transform>,
    pub factions: SparseSet<Faction>,
    pub entity_types: SparseSet<EntityType>,
    pub models: SparseSet<Model>,
    pub ani_models: SparseSet<Model>,
    pub animators: SparseSet<Animator>,
    pub skellingtons: SparseSet<Bone>,
    pub rotators: SparseSet<Rotator>,
    pub inventories: SparseSet<Inventory>,
    pub active_items: SparseSet<ActiveItem>,
    pub item_bones: SparseSet<ItemBones>,
    pub impulse_applied: SparseSet<bool>,
    pub player_controllers: SparseSet<PlayerController>,
    pub simstate_controllers: SparseSet<SimStateController>,
    pub destinations: SparseSet<Vec3>,
    pub cuboids: SparseSet<Cuboid>,
    pub colliders: SparseSet<ColliderShape>,
    pub parents: SparseSet<Parent>,
    pub child_locals: SparseSet<Transform>,
    pub rng: ChaCha8Rng,
    pub selected: Vec<usize>,
    pub v_effects: SparseSet<VisualEffect>,
    pub entity_trashcan: Vec<usize>,
    pub physics_handles: SparseSet<PhysicsHandle>,
    pub collider_to_entity: HashMap<ColliderHandle, usize>,
    pub rigidbody_to_entity: HashMap<RigidBodyHandle, usize>,
    pub hitsets: SparseSet<HashSet<ColliderHandle>>,
    pub yaws: SparseSet<f32>,
    pub knockbacks: SparseSet<Knockback>,
    pub healths: SparseSet<f32>,
    pub base_speeds: SparseSet<f32>,
    pub aggro_ranges: SparseSet<f32>,
}

impl EntityManager {
    pub fn new(max_entities: usize) -> Self {
        Self {
            next_entity_id: 0,
            transforms: SparseSet::with_capacity(max_entities),
            factions: SparseSet::with_capacity(max_entities),
            entity_types: SparseSet::with_capacity(max_entities),
            models: SparseSet::with_capacity(max_entities),
            ani_models: SparseSet::with_capacity(max_entities),
            animators: SparseSet::with_capacity(max_entities),
            skellingtons: SparseSet::with_capacity(max_entities),
            rotators: SparseSet::with_capacity(max_entities),
            inventories: SparseSet::with_capacity(max_entities),
            active_items: SparseSet::with_capacity(max_entities),
            item_bones: SparseSet::with_capacity(max_entities),
            impulse_applied: SparseSet::with_capacity(max_entities),
            player_controllers: SparseSet::with_capacity(max_entities),
            simstate_controllers: SparseSet::with_capacity(max_entities),

            destinations: SparseSet::with_capacity(max_entities),

            cuboids: SparseSet::with_capacity(max_entities),
            // this is just for visuals/debug. The *actual* collider is in 
            // the rapier physics system,tracked by a physics handle.
            colliders: SparseSet::with_capacity(max_entities),

            parents: SparseSet::with_capacity(max_entities),
            child_locals: SparseSet::with_capacity(max_entities),
            rng: ChaCha8Rng::seed_from_u64(1),

            selected: Vec::new(),
            v_effects: SparseSet::with_capacity(max_entities),
            entity_trashcan: Vec::new(),
            physics_handles: SparseSet::with_capacity(max_entities),
            collider_to_entity: HashMap::new(),
            rigidbody_to_entity: HashMap::new(),
            hitsets: SparseSet::with_capacity(max_entities),
            yaws: SparseSet::with_capacity(max_entities),
            knockbacks: SparseSet::with_capacity(max_entities),
            healths: SparseSet::with_capacity(max_entities),
            base_speeds: SparseSet::with_capacity(max_entities),
            aggro_ranges: SparseSet::with_capacity(max_entities),
        }
    }

    pub fn populate_initial_entity_data(&mut self, ec: &EntityConfig, wd: &mut WorldData, ps: &mut PhysicsState) {
        for instance in wd.entities.iter() {
            let archetype = ec.entity_types.get(&instance.entity_type).unwrap();

            match instance.faction {
                Faction::Player | Faction::Enemy => {
                    self.create_animated_entity(
                        ec,
                        instance,
                        ps,
                    );
                },
                Faction::Item => {
                    self.create_static_entity(
                        instance,
                        archetype, 
                        ps,
                        HitboxType::BoundingBox,
                    );
                },
                Faction::World | Faction::Static | Faction::Gizmo => {
                    self.create_static_entity(
                        instance,
                        archetype, 
                        ps,
                        archetype.hitbox_type.clone(),
                    );
                },
            }

        }
    }

    pub fn create_static_entity(
        &mut self, instance: &EntityInstance, 
        archetype: &EntityTypeHelper, 
        ps: &mut PhysicsState,
        hbt: HitboxType,
    ) -> usize {
        let parent_id = self.next_entity_id;
        let position = instance.position;
        let rotation = instance.rotation;
        let scale    = archetype.scale_correction;

        self.factions.insert(self.next_entity_id, instance.faction.clone());
        self.entity_types.insert(self.next_entity_id, instance.entity_type.clone());

        let transform = Transform {
            position,
            rotation: rotation * archetype.rot_correction,
            scale: archetype.scale_correction,

            original_rotation: archetype.rot_correction,
        };
        self.transforms.insert(self.next_entity_id, transform.clone());

        let mut model = Model::new();
        let mut found = false;
        for m in self.models.iter_mut() {
            if m.value().full_path == *archetype.mesh_path.to_string() {
                model = m.value().clone();
                found = true;
            }
        }

        if !found {
            model = import_model_data(&archetype.mesh_path, &Animation::default());
        }
        self.models.insert(self.next_entity_id, model.clone());
        
        self.next_entity_id += 1;

        match hbt {
            HitboxType::Cylinder => {
                self.create_cylinder_hitbox(
                    archetype.hit_cyl.as_ref().unwrap().clone(), 
                    position, 
                    scale, 
                    rotation, 
                    parent_id, 
                    ps
                );
            },
            HitboxType::BoundingBox => {
                self.create_bounding_hitbox(
                    &model,
                    position,
                    scale,
                    rotation,
                    parent_id,
                    ps,
                );
            },
            HitboxType::Mesh => {
                self.create_mesh_based_hitbox(
                    &model,
                    position,
                    scale,
                    rotation,
                    parent_id,
                    ps,
                );
            },
            _ => ()
        }

        parent_id
    }

    pub fn create_animated_entity(
        &mut self,
        ec: &EntityConfig,
        instance: &EntityInstance,
        ps: &mut PhysicsState,
    ) {
        // Reserve an ID for the main entity
        let parent_id = self.next_entity_id;
        self.next_entity_id += 1;

        let position = instance.position;
        let rotation = instance.rotation;
        let faction = &instance.faction;
        let entity_type = &instance.entity_type;
        
        let archetype = ec.entity_types.get(&instance.entity_type).unwrap();
        let model_path = &archetype.mesh_path;
        let bone_path = &archetype.bone_path;
        let rot_correction = archetype.rot_correction;
        let scale = archetype.scale_correction; // We should do this by the instance
        let animation_props = &archetype.animation_properties;
        let item_bones = &archetype.item_bones;
        let cylinder = &archetype.hit_cyl;

        // === TRANSFORM ===
        let transform = Transform {
            position,
            rotation,
            scale,
            original_rotation: rotation,
        };

        self.yaws.insert(parent_id, 0.0);
        self.healths.insert(parent_id, instance.health);

        if let Some(speed) = instance.base_speed {
            self.base_speeds.insert(parent_id, speed);
        }

        // === ANIMATION DATA ===
        let (skellington, mut animator, animation) = import_bone_data(&bone_path, archetype.flip_180);

        for prop in animation_props {
            if let Some(anim) = animator.animations.get_mut(&prop.name) {
                for (k, v) in &prop.one_shots {
                    for frame in v {
                        anim.one_shots.push(OneShot {
                            sound_type: k.clone(),
                            segment: *frame,
                            triggered: false.into(),
                        });
                    }
                }

                for cs in &prop.continuous_sounds {
                    anim.continuous_sounds.push(ContinuousSound {
                        sound_type: cs.clone(),
                        playing: false.into(),
                    });
                }

                if !prop.hurtbox_activation.is_empty() {
                    anim.hurtbox_activation = Some(FrameActivation {
                        segment_range: prop.hurtbox_activation[0]..=prop.hurtbox_activation[1],
                        triggered: false.into(),
                    });
                }
                anim.hold_frame = prop.hold_frame;
            }
        }

        // === MODEL ===
        let model = self.ani_models.iter()
            .find(|m| m.value().full_path == *model_path)
            .map(|m| m.value().clone())
            .unwrap_or_else(|| import_model_data(&model_path, &animation));

        // === ROTATOR ===
        let starting_rot = rotation * rot_correction;
        let rotator = Rotator {
            cur_rot: starting_rot,
            next_rot: starting_rot,
            blend_factor: 0.0,
            blend_time: 0.11,
        };

        // === COMPONENT INSERTION ===
        self.transforms.insert(parent_id, transform);
        self.factions.insert(parent_id, faction.clone());
        self.entity_types.insert(parent_id, entity_type.clone());
        self.animators.insert(parent_id, animator);
        self.skellingtons.insert(parent_id, skellington);
        self.ani_models.insert(parent_id, model);
        self.rotators.insert(parent_id, rotator);
        self.item_bones.insert(parent_id, item_bones.clone());

        if *faction == Faction::Player {
            self.player_controllers.insert(parent_id, PlayerController {
                state: PlayerState::Idle,
                attack_state: AttackState::Attack1,
                time_in_state: 0.0,
            });
        } else {
            self.destinations.insert(parent_id, position);
            self.simstate_controllers.insert(parent_id, SimStateController::default());
            self.aggro_ranges.insert(parent_id, archetype.aggro_range);
        }

        // PILL
        if let Some(cyl) = cylinder {

            let cyl_pos = position;
            // === PHYSICS ===
            let iso: Isometry<f32> = (cyl_pos, rotation).into();
            let mut body = RigidBodyBuilder::dynamic()
                .ccd_enabled(true)
                .position(iso)
                .enabled_rotations(false, false, false)
                .build();

            match entity_type {
                EntityType::YRobot | EntityType::TrashGuy=> {
                    body.set_additional_mass(1.2, false);
                },
                _ => ()
            }
            
            let capsule_total_height = cyl.h;
            let capsule_half_height = (capsule_total_height - 2.0 * cyl.r) / 2.0;

            let offset = 0.039;

            let collider = ColliderBuilder::capsule_y(capsule_half_height, cyl.r)
            // let collider = ColliderBuilder::cylinder(cyl.h * 0.5, cyl.r)
                .active_collision_types(ActiveCollisionTypes::all())
                // TODO: This is a hacky way to fix the fact that colliders are centered at half height
                // by default. Likely there is a better way to fix this?
                .translation(vector![0.0, (capsule_total_height * 0.5) + offset, 0.0]) 
                .build();

            let collider_shape = ColliderShape::capsule_y(capsule_half_height, cyl.r);

            let body_handle = ps.rigid_body_set.insert(body);

            let collider_handle = ps.collider_set.insert_with_parent(
                collider,
                body_handle,
                &mut ps.rigid_body_set,
            );

            let physics_handle = PhysicsHandle {
                rigid_body: body_handle,
                collider: collider_handle,
            };

            self.physics_handles.insert(parent_id, physics_handle);
            self.collider_to_entity.insert(collider_handle, parent_id);
            self.rigidbody_to_entity.insert(body_handle, parent_id);

            // === CYLINDER GIZMO (child entity) ===
            let collider_id = self.next_entity_id;

            let collider_model = if collider_shape.is::<Capsule>() {
                Pill {
                    r: cyl.r,
                    h: capsule_total_height,
                }.create_model(12, 5, offset)
            } else if collider_shape.is::<rapier3d::prelude::Cylinder>() {
                cyl.create_model(12)
            } else {
                println!("Defaulting to cylinder!!!!");
                cyl.create_model(12)
            };

            self.transforms.insert(collider_id, Transform {
                position: cyl_pos,
                rotation: Quat::IDENTITY,
                scale: Vec3::splat(1.0),
                original_rotation: Quat::IDENTITY,
            });
            self.models.insert(collider_id, collider_model);
            self.entity_types.insert(collider_id, EntityType::Cylinder);
            self.factions.insert(collider_id, Faction::Gizmo);
            self.colliders.insert(collider_id, collider_shape);
            self.parents.insert(collider_id, Parent { parent_id: parent_id });

            self.next_entity_id += 1;
        }

        // === WEAPONS ===

        for weapon in instance.weapons.iter() {
            let addtl_weap_rot = if *entity_type == EntityType::YRobot {
                Quat::from_rotation_z(std::f32::consts::PI)
            } else {
                Quat::IDENTITY
            };
            let entity_id = parent_id;
            let weapon_archetype = ec.entity_types.get(&weapon).unwrap();

            let wi = EntityInstance {
                entity_type: weapon.clone(),
                faction: Faction::Item,
                position: Vec3::splat(0.0),
                rotation: addtl_weap_rot * weapon_archetype.rot_correction,
                weapons: vec![],
                base_speed: None,
                health: 100.0,
            };

            let weapon_id = self.create_static_entity(
                &wi, 
                weapon_archetype,
                ps,
                HitboxType::BoundingBox,
            );

            self.hitsets.insert(
                weapon_id,
                HashSet::new(),
            );
            
            self.parents.insert(weapon_id, Parent { parent_id: entity_id });
            if let Some(_) = self.active_items.get(entity_id) {
                if let Some(inv) = self.inventories.get_mut(entity_id) {
                    inv.items.push(weapon_id);
                } else {
                    self.inventories.insert(entity_id, Inventory { items: vec![weapon_id] });
                }
            } else {
                self.active_items.insert(
                    entity_id,
                    ActiveItem {
                        right_hand: Some(weapon_id),
                        left_hand: None,
                    }
                );
            }

        }
    }

    pub fn create_cylinder_hitbox(
        &mut self, 
        cyl: Cylinder,
        position: Vec3,
        scale: Vec3,
        rotation: Quat,
        parent_id: usize,
        ps: &mut PhysicsState,
    ) {
        let cyl_mod = cyl.create_model(12);

        let collider_vert_dim = (cyl.h * 0.5) - 0.025;

        let collider_shape = ColliderShape::cylinder(collider_vert_dim, cyl.r);

        self.colliders.insert(self.next_entity_id, collider_shape);

        self.models.insert(self.next_entity_id, cyl_mod);
        self.factions.insert(self.next_entity_id, Faction::Gizmo);
        self.entity_types.insert(self.next_entity_id, EntityType::Cylinder);
        self.transforms.insert(self.next_entity_id, Transform {
            position,
            rotation: Quat::IDENTITY,
            scale,
            original_rotation: Quat::IDENTITY,
        });

        self.parents.insert(self.next_entity_id, Parent{
            parent_id,
        });

        let iso: Isometry<f32> = (position, rotation).into();

        let body = RigidBodyBuilder::fixed()
            .position(iso)
            .build();

        let collider = ColliderBuilder::cylinder(cyl.h * 0.5, cyl.r)
            .active_collision_types(ActiveCollisionTypes::all())
            .build();

        let body_handle = ps.rigid_body_set.insert(body);
        let collider_handle = ps.collider_set.insert_with_parent(
            collider,
            body_handle,
            &mut ps.rigid_body_set,
        );

        self.physics_handles.insert(self.next_entity_id, PhysicsHandle {
            rigid_body: body_handle,
            collider: collider_handle,
        });

        self.collider_to_entity.insert(collider_handle, parent_id);
        self.rigidbody_to_entity.insert(body_handle, parent_id);

        self.next_entity_id += 1;

    }

    pub fn create_bounding_hitbox(
        &mut self,
        model: &Model,
        position: Vec3,
        scale: Vec3,
        rotation: Quat,
        parent_id: usize,
        ps: &mut PhysicsState,
    ) {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);

        for v in model.vertices.iter() {
            min = min.min(v.position);
            max = max.max(v.position);
        }

        let size = max - min;
        let center = (min + max) * 0.5;

        let mut local_offset = center;

        local_offset.y = 0.5 * size.y;

        let cuboid = Cuboid {
            w: size.x,
            h: size.y,
            d: size.z,
        };

        let cuboid_model = cuboid.create_model();

        self.cuboids.insert(self.next_entity_id, cuboid);
        self.models.insert(self.next_entity_id, cuboid_model);
        self.factions.insert(self.next_entity_id, Faction::Gizmo);
        self.entity_types.insert(self.next_entity_id, EntityType::Cuboid);
        self.parents.insert(self.next_entity_id, Parent { parent_id });
        self.child_locals.insert(self.next_entity_id, Transform {
            position,
            rotation: Quat::IDENTITY,
            scale,
            original_rotation: Quat::IDENTITY
        });
        self.transforms.insert(self.next_entity_id, Transform {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale,
            original_rotation: Quat::IDENTITY,
        });

        // PHYSICS PASS
        let iso: Isometry<f32> = (position, rotation).into();

        let body = RigidBodyBuilder::kinematic_position_based()
            .position(iso)
            .ccd_enabled(true)
            .build();

        let half_extents = size * 0.5;

        let collider_shape = ColliderShape::cuboid(half_extents.x, half_extents.y, half_extents.z);
        self.colliders.insert(self.next_entity_id, collider_shape);

        let collider = ColliderBuilder::cuboid(size.x * 0.5, size.y * 0.5, size.z * 0.5)
            .translation(vector![0.0, half_extents.y, 0.0])
            .sensor(true)
            .density(0.0)
            .active_events(ActiveEvents::COLLISION_EVENTS)
            .build();

        let body_handle = ps.rigid_body_set.insert(body);
        let collider_handle = ps.collider_set.insert_with_parent(
            collider,
            body_handle,
            &mut ps.rigid_body_set,
        );
        self.physics_handles.insert(parent_id, PhysicsHandle {
            rigid_body: body_handle,
            collider: collider_handle,
        });

        self.collider_to_entity.insert(collider_handle, parent_id);
        self.rigidbody_to_entity.insert(body_handle, parent_id);

        self.next_entity_id += 1;
    }

    pub fn create_mesh_based_hitbox(
        &mut self,
        model: &Model,
        position: Vec3,
        scale: Vec3,
        rotation: Quat,
        parent_id: usize,
        ps: &mut PhysicsState,
    ) {

        println!("creating mesh based hitbox");
        // Process vertices into arrays
        let vertices: Vec<Point3<f32>> = model.vertices
            .iter()
            .map(|v| v.position.into())
            .collect();
        
        let indices: Vec<[u32; 3]> = model.indices
            .chunks(3)
            .map(|chunk| [chunk[0], chunk[1], chunk[2]])
            .collect();

        // let collider_shape = ColliderShape::trimesh(vertices, indices).unwrap();

        // self.colliders.insert(self.next_entity_id, collider_shape);

        self.transforms.insert(self.next_entity_id, Transform {
            position,
            rotation: Quat::IDENTITY,
            scale,
            original_rotation: Quat::IDENTITY,
        });

        let iso: Isometry<f32> = (position, rotation).into();

        let body = RigidBodyBuilder::fixed()
            .position(iso)
            .build();

        let collider = ColliderBuilder::trimesh(vertices, indices).unwrap()
            .active_collision_types(ActiveCollisionTypes::all())
            .build();

        let body_handle = ps.rigid_body_set.insert(body);
        let collider_handle = ps.collider_set.insert_with_parent(
            collider,
            body_handle,
            &mut ps.rigid_body_set,
        );

        self.physics_handles.insert(self.next_entity_id, PhysicsHandle {
            rigid_body: body_handle,
            collider: collider_handle,
        });

        self.next_entity_id += 1;
    }

    pub fn update(&mut self, sm: &mut SoundManager, ps: &mut PhysicsState) {

        let dynamic_rbs: Vec<(RigidBodyHandle, &RigidBody)> = ps.rigid_body_set
            .iter()
            .filter(|rb| rb.1.body_type().is_dynamic())
            .map(|rb| rb)
            .collect();

        let k_pos_based_rbs: Vec<RigidBodyHandle> = ps.rigid_body_set
            .iter()
            .filter(|rb| rb.1.body_type() == RigidBodyType::KinematicPositionBased)
            .map(|rb| rb.0)
            .collect();

        for rb in dynamic_rbs.iter() {
            let entity_id = self.rigidbody_to_entity.get(&rb.0).unwrap();

            if let Some(transform) = self.transforms.get_mut(*entity_id) {
                let iso = rb.1.position();

                transform.position = Vec3::new(iso.translation.x, iso.translation.y, iso.translation.z);
                transform.rotation = Quat::from_xyzw(
                    iso.rotation.i,
                    iso.rotation.j,
                    iso.rotation.k,
                    iso.rotation.w,
                );
            }
        }


        for rbh in k_pos_based_rbs.iter() {
            let entity_id = self.rigidbody_to_entity.get(rbh).unwrap();

            let rb = ps.rigid_body_set.get_mut(*rbh).unwrap();

            let t = self.transforms.get(*entity_id).unwrap();

             
            let iso = Isometry::from_parts(
                Translation::from(vector![t.position.x, t.position.y, t.position.z]),
                UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(t.rotation.w, t.rotation.x, t.rotation.y, t.rotation.z)),
            );
            rb.set_next_kinematic_position(iso);
        }

        self.apply_parenting();
        self.delete_entities(sm);
    }

   pub fn delete_entities(&mut self, sm: &mut SoundManager) {
       // TODO: Also clean up colliders from here.
       for id in self.entity_trashcan.iter() {
           // sm.cleanup_entity_sounds(*id);
           self.active_items.remove(*id);
           self.inventories.remove(*id);
           self.transforms.remove(*id);
           self.factions.remove(*id);
           self.entity_types.remove(*id);
           self.models.remove(*id);
           self.ani_models.remove(*id);
           self.animators.remove(*id);
           self.skellingtons.remove(*id);
           self.rotators.remove(*id);
           self.simstate_controllers.remove(*id);
           self.destinations.remove(*id);
           self.parents.remove(*id);
           self.colliders.remove(*id);
           self.v_effects.remove(*id);
           self.impulse_applied.remove(*id);
       }

       self.entity_trashcan.clear();
   }

    pub fn get_ids_for_faction(&self, faction: Faction) -> Vec<usize> {
        let result: Vec<usize> = self.factions
            .iter()
            .filter_map(|f|
                if *f.value() == faction {
                    Some(f.key())
                } else {
                    None
                }
            )
            .collect();

        result
    }

    pub fn get_ids_for_type(&self, entity_type: EntityType) -> Vec<usize> {
        let result: Vec<usize> = self.entity_types
            .iter()
            .filter_map(|f|
                if *f.value() == entity_type {
                    Some(f.key())
                } else {
                    None
                }
            )
            .collect();

        result
    }

    pub fn player_get_ids_for_state(&self, state: PlayerState) -> Vec<usize> {
        let result: Vec<usize> = self.player_controllers
            .iter()
            .filter_map(|f|
                if f.value().state == state {
                    Some(f.key())
                } else {
                    None
                }
            )
            .collect();

        result

    }

    pub fn enemy_get_ids_for_state(&self, state: SimState) -> Vec<usize> {
        let result: Vec<usize> = self.simstate_controllers
            .iter()
            .filter_map(|f|
                if f.value().state == state {
                    Some(f.key())
                } else {
                    None
                }
            )
            .collect();

        result

    }

    pub fn get_active_weapon_ids(&self) -> Vec<usize> {
        self.active_items
            .iter()
            .flat_map(|item| {
                let active = item.value();
                [active.right_hand, active.left_hand]
                    .into_iter()
                    .flatten()
            })
            .collect()
    }

    pub fn get_all_orphaned_weapon_ids(&self) -> Vec<usize> {
        self.factions
            .iter()
            .filter(|w_type| {
                *w_type.value() == Faction::Item 
                    && self.active_items.get(w_type.key()).is_none()
                    && self.parents.get(w_type.key()).is_none()
            })
            .map(|e| e.key())
            .collect::<Vec<usize>>()
    }

    pub fn apply_parenting(&mut self) {
        // NOTE: This handles one-level parenting. If you have deep hierarchies,
        // sort by depth or recurse.
        // Here, your cuboid is just one level under the weapon, so this is fine.
        let mut to_update: Vec<(usize, usize)> = Vec::new();
        for p in self.parents.iter() {
            to_update.push((p.key(), p.value().parent_id)); // (child, parent)
        }

        for (child, parent) in to_update {
            // Parent world
            let pt = match self.transforms.get(parent) {
                Some(t) => t.clone(),
                None => continue, // parent lacks a world; skip
            };
            let parent_world =
            Mat4::from_scale_rotation_translation(pt.scale, pt.rotation, pt.position);

            // Child local (must exist)
            let lt = match self.child_locals.get(child) {
                Some(t) => t.clone(),
                None => continue, // no local stored; skip
            };
            let child_local =
            Mat4::from_scale_rotation_translation(lt.scale, lt.rotation, lt.position);

            let world = parent_world * child_local; // compose

            let (s, r, p) = world.to_scale_rotation_translation();
            let ct = self.transforms.get_mut(child).unwrap(); // guaranteed present
            ct.position = p;
            ct.rotation = r;
            ct.scale = s;
            // Keep ct.original_rotation as-is (unused for gizmo)
        }
    }
}

pub fn glam_to_nalgebra_quat(q: Quat) -> UnitQuaternion<f32> {
    UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(q.w, q.x, q.y, q.z))
}
