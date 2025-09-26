#![allow(clippy::too_many_arguments)]

use std::collections::HashSet;

use glam::{Mat4, Quat, Vec3};
use nalgebra::UnitQuaternion;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rapier3d::{parry::{shape::Capsule, utils::hashmap::HashMap}, prelude::*};

use crate::{animation::animation::{import_bone_data, import_model_data, Animation, Animator, Bone, Model}, config::{entity_config::{AnimationPropHelper, EntityConfig, ItemBones}, world_data::WorldData}, debug::gizmos::{Cuboid, Pill}, enums_types::{ActiveItem, EntityType, Faction, FrameActivation, Inventory, Knockback, Parent, PhysicsHandle, PlayerController, PlayerState, Rotator, SimState, SimStateController, Transform, VisualEffect}, physics::PhysicsState, sound::sound_manager::{ContinuousSound, OneShot, SoundManager}, sparse_set::SparseSet};

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

    // Simulation/Behavior Components
    pub destinations: SparseSet<Vec3>,

    // Simulation gizmos
    pub cuboids: SparseSet<Cuboid>,
    pub colliders: SparseSet<ColliderShape>,
    // pub cylinders: SparseSet<Cylinder>,

    pub parents: SparseSet<Parent>,
    pub child_locals: SparseSet<Transform>,
    pub rng: ChaCha8Rng,

    pub selected: Vec<usize>,
    pub v_effects: SparseSet<VisualEffect>,
    pub entity_trashcan: Vec<usize>,

    // Physics stuff
    // Find colliders from entities
    pub physics_handles: SparseSet<PhysicsHandle>,
    // Find entities from rapier
    pub collider_to_entity: HashMap<ColliderHandle, usize>,
    pub rigidbody_to_entity: HashMap<RigidBodyHandle, usize>,
    pub hitsets: SparseSet<HashSet<ColliderHandle>>,
    pub yaws: SparseSet<f32>,
    pub knockbacks: SparseSet<Knockback>,
    pub healths: SparseSet<f32>,
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
            // cylinders: SparseSet::with_capacity(max_entities),

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
        }
    }

    pub fn populate_initial_entity_data(&mut self, ec: &mut EntityConfig, wd: &mut WorldData, ps: &mut PhysicsState) {
        for instance in wd.entities.iter() {
            let archetype = ec.entity_types.get(&instance.entity_type).unwrap();
            let position = instance.position;
            let rotation = instance.rotation;
            let scale_correction = archetype.scale_correction;

            let rot_correction = match archetype.rot_correction.as_str() {
                "-FRAC_PI_2" => Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                _ => Quat::IDENTITY,
            };
            match instance.faction {
                Faction::Player | Faction::Enemy => {
                    self.create_animated_entity(
                        instance.faction.clone(),
                        position.into(), 
                        scale_correction.into(), 
                        rot_correction, 
                        Quat::from_xyzw(rotation[0], rotation[1], rotation[2], rotation[3]),
                        &archetype.mesh_path, 
                        &archetype.bone_path,
                        &archetype.animation_properties,
                        instance.entity_type.clone(),
                        archetype.hit_cyl.clone(),
                        ps,
                        archetype.flip_180.clone(),
                        archetype.item_bones.clone(),
                    );
                },
                Faction::World | Faction::Static | Faction::Gizmo | Faction::Item => {
                    self.create_static_entity(
                        instance.entity_type.clone(),
                        instance.faction.clone(),
                        position.into(), 
                        scale_correction.into(), 
                        rot_correction, 
                        Quat::from_xyzw(rotation[0], rotation[1], rotation[2], rotation[3]),
                        &archetype.mesh_path, 
                        archetype.hit_cyl.clone(),
                        ps,
                    );
                },
            }

        }


        {
            // Load a weapon for the player // TODO: don't hard code this
            let player_id = self.factions.iter().filter(|f| *f.value() == Faction::Player).last().unwrap().key();
            let weapon_id = self.next_entity_id;
            self.create_static_entity(
                EntityType::OrcSword, 
                Faction::Item, 
                Vec3::splat(0.0), 
                Vec3::splat(1.0), 
                // Quat::IDENTITY,
                // 90 about y and then -90 about z, this gives us a perpendicular weapon.
                // Quat::from_rotation_z(std::f32::consts::PI) * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2) * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                Quat::IDENTITY, 
                "resources/models/static/weapons/swords/001_double_axe_new.txt", 
                None,
                ps,
            );

            self.active_items.insert(
                player_id,
                ActiveItem {
                    right_hand: Some(weapon_id),
                    left_hand: None,
                }
            );

            self.hitsets.insert(
                weapon_id,
                HashSet::new(),
            );
        }

        {
            // Load an inventory weapon for the player // TODO: don't hard code this
            let player_id = self.factions.iter().filter(|f| *f.value() == Faction::Player).last().unwrap().key();
            let weapon_id = self.next_entity_id;
            self.create_static_entity(
                EntityType::OrcSword, 
                Faction::Item, 
                Vec3::splat(0.0), 
                Vec3::splat(1.0), 
                // Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                Quat::from_rotation_x(std::f32::consts::PI) * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                Quat::IDENTITY, 
                "resources/models/static/weapons/001_orc_sword_bc.txt", 
                None,
                ps,
            );

            self.inventories.insert(
                player_id,
                Inventory {
                    items: vec![weapon_id],
                },
            );

            self.hitsets.insert(
                weapon_id,
                HashSet::new(),
            );
        }
    }

    pub fn create_static_entity(&mut self,entity_type: EntityType, faction: Faction, position: Vec3, scale: Vec3, rot_correction: Quat,rotation: Quat, model_path: &str, cylinder: Option<crate::debug::gizmos::Cylinder>, ps: &mut PhysicsState) {

        let parent_id = self.next_entity_id;

        self.factions.insert(self.next_entity_id, faction);
        self.entity_types.insert(self.next_entity_id, entity_type);

        let transform = Transform {
            position,
            rotation: rotation * rot_correction,
            scale,

            original_rotation: rot_correction,
        };
        self.transforms.insert(self.next_entity_id, transform.clone());

        self.yaws.insert(parent_id, 0.0);

        let mut model = Model::new();
        let mut found = false;
        for m in self.models.iter_mut() {
            if m.value().full_path == *model_path.to_string() {
                model = m.value().clone();
                found = true;
            }
        }

        if !found {
            model = import_model_data(model_path, &Animation::default());
        }
        self.models.insert(self.next_entity_id, model.clone());
        
        self.next_entity_id += 1;
        
        // ============================================================
        // CREATE CUBE BOUNDING BOX
        // ============================================================
        {
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
            self.physics_handles.insert(self.next_entity_id, PhysicsHandle {
                rigid_body: body_handle,
                collider: collider_handle,
            });

            self.collider_to_entity.insert(collider_handle, self.next_entity_id);
            self.rigidbody_to_entity.insert(body_handle, self.next_entity_id);

            self.next_entity_id += 1;
        }

        // ============================================================
        // CREATE CYLINDER HITBOX
        // ============================================================
        {
            if let Some(cyl) = cylinder {
                // CYLINDER PASS
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

                // PHYSICS PASS
                let iso: Isometry<f32> = (position, rotation).into();
                
                // TODO: This shouldn't be fixed always, we can have it be kinematic for some
                // things
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

                self.collider_to_entity.insert(collider_handle, self.next_entity_id);
                self.rigidbody_to_entity.insert(body_handle, self.next_entity_id);

                self.next_entity_id += 1;
            }
        }
    }

    pub fn create_animated_entity(
        &mut self,
        faction: Faction,
        position: Vec3,
        scale: Vec3,
        rot_correction: Quat,
        rotation: Quat,
        model_path: &str,
        animation_path: &str,
        animation_props: &[AnimationPropHelper],
        entity_type: EntityType,
        cylinder: Option<crate::debug::gizmos::Cylinder>,
        ps: &mut PhysicsState,
        flip_180: bool,
        item_bones: ItemBones,
    ) {
        // Reserve an ID for the main entity
        let entity_id = self.next_entity_id;
        self.next_entity_id += 1;

        // === TRANSFORM ===
        let transform = Transform {
            position,
            rotation,
            scale,
            original_rotation: rotation,
        };

        self.yaws.insert(entity_id, 0.0);
        self.healths.insert(entity_id, 100.0);

        // === ANIMATION DATA ===
        let (skellington, mut animator, animation) = import_bone_data(animation_path, flip_180);

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
            }
        }

        // === MODEL ===
        let model = self.ani_models.iter()
            .find(|m| m.value().full_path == model_path)
            .map(|m| m.value().clone())
            .unwrap_or_else(|| import_model_data(model_path, &animation));

        // === ROTATOR ===
        let starting_rot = rotation * rot_correction;
        let rotator = Rotator {
            cur_rot: starting_rot,
            next_rot: starting_rot,
            blend_factor: 0.0,
            blend_time: 0.11,
        };

        // === COMPONENT INSERTION ===
        self.transforms.insert(entity_id, transform);
        self.factions.insert(entity_id, faction.clone());
        self.entity_types.insert(entity_id, entity_type.clone());
        self.animators.insert(entity_id, animator);
        self.skellingtons.insert(entity_id, skellington);
        self.ani_models.insert(entity_id, model);
        self.rotators.insert(entity_id, rotator);
        self.item_bones.insert(entity_id, item_bones);
        self.simstate_controllers.insert(entity_id, match entity_type {
            EntityType::MooseMan => { 
                SimStateController {
                    state: SimState::Dancing,
                    time_in_state: 0.0,
                }
            },
            _ => {
                SimStateController {
                    state: SimState::Waiting,
                    time_in_state: 0.0,
                }
            },
        });

        if faction == Faction::Player {
            self.player_controllers.insert(entity_id, PlayerController {
                state: PlayerState::Idle,
                time_in_state: 0.0,
            });
        } else {
            self.destinations.insert(entity_id, position);
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
                EntityType::YRobot => {
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

            self.physics_handles.insert(entity_id, physics_handle);
            self.collider_to_entity.insert(collider_handle, entity_id);
            self.rigidbody_to_entity.insert(body_handle, entity_id);

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
            self.parents.insert(collider_id, Parent { parent_id: entity_id });

            self.next_entity_id += 1;
        }
    }

    pub fn update(&mut self, sm: &mut SoundManager, ps: &mut PhysicsState) {
        self.delete_entities(sm);

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

            let set_iso = if let Some(parent) = self.parents.get(*entity_id) {
                // RB should live at the parent's origin; collider's own local .translation handles the lift.
                if let Some(pt) = self.transforms.get(parent.parent_id) {
                    Some(Isometry::from_parts(
                        Translation::from(vector![pt.position.x, pt.position.y, pt.position.z]),
                        UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(pt.rotation.w, pt.rotation.x, pt.rotation.y, pt.rotation.z)),
                    ))
                } else { None }
            } else if let Some(t) = self.transforms.get(*entity_id) {
                // if no parent, drive from the entity itself
                Some(Isometry::from_parts(
                    Translation::from(vector![t.position.x, t.position.y, t.position.z]),
                    UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(t.rotation.w, t.rotation.x, t.rotation.y, t.rotation.z)),
                ))
            } else {
                None
            };

            if let Some(iso) = set_iso {
                rb.set_next_kinematic_position(iso);
            }
        }

        // for rb in k_pos_based_rbs.iter() {
        //     let entity_id = self.rigidbody_to_entity.get(rb).unwrap();

        //     if let Some(transform) = self.transforms.get_mut(*entity_id) {
        //         let iso: Isometry<f32> = (transform.position, transform.rotation).into();
        //         let rb = ps.rigid_body_set.get_mut(*rb).unwrap();
        //         
        //         // TODO: should we wake up or does it not matter?
        //         rb.set_position(iso, true);
        //     }

        // }

        self.apply_parenting();
    }

    pub fn delete_entities(&mut self, sm: &mut SoundManager) {
        // TODO: Also clean up colliders from here.
        for id in self.entity_trashcan.iter() {
            sm.cleanup_entity_sounds(*id);
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
