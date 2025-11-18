#![allow(clippy::too_many_arguments)]

use core::f32;
use std::collections::{HashMap, HashSet};

use glam::{Mat4, Quat, Vec3};
use nalgebra::{Point3, UnitQuaternion, Vector3};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rapier3d::prelude::*;

use crate::{animation::{self, animation::{Animation, Animator, Bone, Model}}, config::{entity_config::{AnimationPropHelper, EntityConfig, EntityTypeHelper, ItemBones}, world_data::{EntityInstance, WorldData}}, debug::gizmos::{Cuboid, Cylinder, Pill}, enums_types::{ActiveItem, AttackState, EntityType, EquipSlot, Faction, FrameActivation, GroundedState, HitboxShape, Inventory, JumpHeight, Knockback, Parent, PhysicsHandle, PlayerController, PlayerState, Rotator, SimState, SimStateController, Transform, VisualEffect}, input::InputState, physics::{self, PhysicsState}, some_data::{GRAVITY, GROUP_PLAYER}, sound::sound_manager::{ContinuousSound, OneShot, SoundManager}, sparse_set::{Entry, SparseSet}, terrain::{self, Terrain}};

pub struct EntityManager {
    pub next_entity_id: usize,
    pub transforms: SparseSet<Transform>,
    pub prev_transforms: SparseSet<Transform>,
    // This is pretty much exclusively for weapons that need an additional 90° orientation for instance
    pub local_corrections: SparseSet<Transform>,
    // The model for rendering the colldier. Otherwwise this is just managed in rapier3d
    pub collider_gizmos: SparseSet<Model>,
    pub collider_transforms: SparseSet<Transform>,
    pub prev_collider_transforms: SparseSet<Transform>,
    pub collider_to_entity: HashMap<ColliderHandle, usize>,

    pub inventories: SparseSet<Vec<usize>>,
    pub active_items: SparseSet<ActiveItem>,

    // This is for a weapon to "know" that it is in an inventory, it's a little messy but we just
    // have to be careful to remove them properly.
    pub owners: SparseSet<usize>,
    pub is_equipped: SparseSet<bool>,
    // sockets for items to attach to, parent to a bone instead of the parent of the bone
    pub item_bones: SparseSet<ItemBones>,
    pub factions: SparseSet<String>,
    pub entity_types: SparseSet<String>,
    pub models: SparseSet<Model>,
    pub animators: SparseSet<Animator>,
    pub skellingtons: SparseSet<Bone>,
    pub rotators: SparseSet<Rotator>,
    pub impulse_applied: SparseSet<bool>,
    pub player_controllers: SparseSet<PlayerController>,
    pub simstate_controllers: SparseSet<SimStateController>,
    pub destinations: SparseSet<Vec3>,
    pub rng: ChaCha8Rng,
    pub selected: Vec<usize>,
    pub v_effects: SparseSet<VisualEffect>,
    pub entity_trashcan: Vec<usize>,
    pub physics_handles: SparseSet<PhysicsHandle>,
    pub hitsets: SparseSet<HashSet<ColliderHandle>>,
    pub yaws: SparseSet<f32>,
    pub knockbacks: SparseSet<Knockback>,
    pub healths: SparseSet<f32>,
    pub base_speeds: SparseSet<f32>,
    pub aggro_ranges: SparseSet<f32>,
    pub jump_heights: SparseSet<JumpHeight>,
    pub total_masses: SparseSet<f32>,
    pub model_heights: SparseSet<f32>,
    pub grounded_states: SparseSet<GroundedState>,

    pub entity_type_register: HashMap<String, EntityTypeHelper>,
    pub faction_register: HashSet<String>,

    pub serializable_world_data: WorldData,
}

impl EntityManager {
    pub fn new(max_entities: usize) -> Self {
        let wd = WorldData::load_from_file("config/world_data.json");

        Self {
            next_entity_id: 0,
            transforms: SparseSet::with_capacity(max_entities),
            prev_transforms: SparseSet::with_capacity(max_entities),
            local_corrections: SparseSet::with_capacity(max_entities),
            collider_gizmos: SparseSet::with_capacity(max_entities),
            collider_transforms: SparseSet::with_capacity(max_entities),
            prev_collider_transforms: SparseSet::with_capacity(max_entities),
            collider_to_entity: HashMap::new(),
            inventories: SparseSet::with_capacity(max_entities),
            active_items: SparseSet::with_capacity(max_entities),
            owners: SparseSet::with_capacity(max_entities),
            is_equipped: SparseSet::with_capacity(max_entities),
            item_bones: SparseSet::with_capacity(max_entities),
            factions: SparseSet::with_capacity(max_entities),
            entity_types: SparseSet::with_capacity(max_entities),
            models: SparseSet::with_capacity(max_entities),
            animators: SparseSet::with_capacity(max_entities),
            skellingtons: SparseSet::with_capacity(max_entities),
            rotators: SparseSet::with_capacity(max_entities),
            impulse_applied: SparseSet::with_capacity(max_entities),
            player_controllers: SparseSet::with_capacity(max_entities),
            simstate_controllers: SparseSet::with_capacity(max_entities),
            destinations: SparseSet::with_capacity(max_entities),
            rng: ChaCha8Rng::seed_from_u64(1),
            selected: Vec::new(),
            v_effects: SparseSet::with_capacity(max_entities),
            entity_trashcan: Vec::new(),
            physics_handles: SparseSet::with_capacity(max_entities),
            hitsets: SparseSet::with_capacity(max_entities),
            yaws: SparseSet::with_capacity(max_entities),
            knockbacks: SparseSet::with_capacity(max_entities),
            healths: SparseSet::with_capacity(max_entities),
            base_speeds: SparseSet::with_capacity(max_entities),
            aggro_ranges: SparseSet::with_capacity(max_entities),
            jump_heights: SparseSet::with_capacity(max_entities),
            total_masses: SparseSet::with_capacity(max_entities),
            model_heights: SparseSet::with_capacity(max_entities),
            grounded_states: SparseSet::with_capacity(max_entities),
            
            // TODO: Probably just return the entity_types here instead of accessing them like this
            entity_type_register: EntityConfig::load_from_file("config/entity_config.json").entity_types,
            faction_register: HashSet::new(),
            serializable_world_data: wd,
        }
    }

    pub fn populate_entity_data(&mut self, ps: &mut PhysicsState) {
        let data = self.serializable_world_data.clone();

        for instance in data.entities.iter() {
            let parent_id = self.create_entity(instance, ps);
            self.populate_inventory(parent_id, &instance, ps);
        }

        load_terrain(self, ps);
    }

    pub fn populate_inventory(&mut self, parent_id: usize, instance: &EntityInstance, ps: &mut PhysicsState) {
        if let Some(weapons_list) = &instance.weapons {
            for weapon in weapons_list.iter() {
                let weapon_id = self.create_entity(
                    weapon, 
                    ps,
                );
                
                self.hitsets.insert(
                    weapon_id,
                    HashSet::new(),
                );

                self.owners.insert(weapon_id, parent_id);

                match self.inventories.get_mut(parent_id) {
                    Some(inv) => {
                        if !inv.contains(&weapon_id) {
                            inv.push(weapon_id);
                        }
                    },
                    None => {
                        let inv = vec![weapon_id];
                        self.inventories.insert(parent_id, inv);
                    }
                }

                match self.active_items.get_mut(parent_id) {
                    Some(_) => (),
                    None => {
                        self.active_items.insert(parent_id, ActiveItem {
                            right_hand: Some(weapon_id),
                            left_hand: None,
                        });

                        if let Some(inv) = self.inventories.get_mut(parent_id) {
                            inv.retain(|v| *v != weapon_id);
                        }

                        self.is_equipped.insert(weapon_id, true);
                    }
                }

            }
        }
    }

    pub fn create_entity(
        &mut self, 
        instance: &EntityInstance, 
        ps: &mut PhysicsState,
    ) -> usize {
        let parent_id = self.next_entity_id;

        let archetype = match self.entity_type_register.get(&instance.entity_type) {
            Some(a) => a,
            None => {
                dbg!(&instance.entity_type);
                panic!();
            }
        };
        let position = instance.position;
        let rotation = instance.rotation;
        let scale    = archetype.scale_correction;

        match instance.faction.as_str() {
            "Player" => {
                self.player_controllers.insert(parent_id, PlayerController {
                    state: PlayerState::Init,
                    attack_state: AttackState::Attack1,
                    time_in_state: 0.0,
                });
            },
            "Enemy" => {
                self.simstate_controllers.insert(parent_id, SimStateController { 
                    state: SimState::Init, 
                    attack_state: AttackState::Attack1, 
                    time_in_state: 0.0,
                    target_time: 0.0, 
                });

                self.destinations.insert(parent_id, position);
                self.aggro_ranges.insert(parent_id, archetype.aggro_range);
            },
            _=> ()
        }

        self.factions.insert(parent_id, instance.faction.clone());
        self.faction_register.insert(instance.faction.clone());
        self.entity_types.insert(parent_id, instance.entity_type.clone());
        self.yaws.insert(parent_id, 0.0);

        if let Some(health) = instance.health {
            self.healths.insert(parent_id, health);
        }

        if let Some(base_speed) = instance.base_speed {
            self.base_speeds.insert(parent_id, base_speed);
        }

        if let Some(jump_height) = instance.jump_height {
            self.jump_heights.insert(parent_id, JumpHeight { desired: jump_height, precalculated: None });
        }

        if let Some(total_mass) = archetype.total_mass {
            self.total_masses.insert(parent_id, total_mass);
        }

        let transform = Transform {
            position,
            rotation,
            scale,
        };
        self.transforms.insert(parent_id, transform.clone());

        // CHECK FOR BONES
        let model = if let (Some(bone_path), Some(anim_props)) = (&archetype.bone_path, &archetype.animation_properties) {
            let (skellington, mut animator, animation) = animation::animation::import_bone_data(bone_path, false);

            for prop in anim_props {
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

            let model = self.models.iter()
                .find(|m| m.value().full_path == *archetype.mesh_path)
                .map(|m| m.value().clone())
                .unwrap_or_else(|| animation::animation::import_model_data(&archetype.mesh_path, &animation));

            let rotator = Rotator {
                cur_rot: instance.rotation,
                next_rot: instance.rotation,
                blend_factor: 0.0,
                blend_time: 0.11,
            };

            self.animators.insert(parent_id, animator);
            self.skellingtons.insert(parent_id, skellington);
            self.models.insert(parent_id, model.clone());
            self.rotators.insert(parent_id, rotator);

            model
        } else {
            let model = self.models.iter()
                .find(|m| m.value().full_path == *archetype.mesh_path)
                .map(|m| m.value().clone())
                .unwrap_or_else(|| animation::animation::import_model_data(&archetype.mesh_path, &Animation::default()));

            self.models.insert(parent_id, model.clone());

            model
        };

        self.prev_transforms.insert(parent_id, transform);
        self.factions.insert(parent_id, instance.faction.clone());
        self.entity_types.insert(parent_id, instance.entity_type.clone());
        self.item_bones.insert(parent_id, archetype.item_bones.clone());

        self.local_corrections.insert(parent_id, Transform {
            position: glam::Vec3::splat(0.0),
            scale: glam::Vec3::splat(1.0),
            rotation: archetype.rot_correction,
        });
        
        self.next_entity_id += 1;

        match archetype.hitbox {
            HitboxShape::Cylinder { r, h } => {
                self.create_cylinder_hitbox(
                    r,
                    h,
                    position, 
                    scale, 
                    rotation, 
                    parent_id, 
                    ps
                );
            },
            HitboxShape::Pill { r, h } => {
                self.create_pill_hitbox(
                    r,
                    h,
                    position, 
                    scale, 
                    rotation, 
                    parent_id, 
                    ps
                );
            },
            HitboxShape::BoundingBox => {
                self.create_bounding_hitbox(
                    &model,
                    position,
                    scale,
                    rotation,
                    parent_id,
                    ps,
                );
            },
            HitboxShape::Mesh => {
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

    pub fn create_pill_hitbox(
        &mut self, 
        r: f32,
        h: f32,
        position: Vec3,
        scale: Vec3,
        rotation: Quat,
        parent_id: usize,
        ps: &mut PhysicsState,
    ) {
        let pill_pos = position;
        // === PHYSICS ===
        let iso: Isometry<f32> = (pill_pos, rotation).into();
        let body = RigidBodyBuilder::dynamic()
            .ccd_enabled(true)
            .position(iso)
            .enabled_rotations(false, false, false)
            .build();
        //let body = RigidBodyBuilder::kinematic_position_based()
        //    .ccd_enabled(true)
        //    .position(iso)
        //    .enabled_rotations(true, true, true)
        //    .build();


        let capsule_total_height = h;
        let capsule_half_height = (capsule_total_height - 2.0 * r) / 2.0;

        let offset = 0.039;

        let builder = ColliderBuilder::capsule_y(capsule_half_height, r)
            // let collider = ColliderBuilder::cylinder(cyl.h * 0.5, cyl.r)
            .active_collision_types(ActiveCollisionTypes::all())
            // TODO: This is a hacky way to fix the fact that colliders are centered at half height
            // by default. Likely there is a better way to fix this?
            .translation(vector![0.0, capsule_half_height + r, 0.0]) 
            .restitution(0.0)
            .restitution_combine_rule(CoefficientCombineRule::Min)
            .friction(2.0)
            .collision_groups(InteractionGroups::new(GROUP_PLAYER.into(), u32::MAX.into()))
            .friction_combine_rule(CoefficientCombineRule::Max);

        let collider = if let Some(mass) = self.total_masses.get(parent_id) {
            builder.mass(*mass).build()
        } else {
            builder.build()
        };

        let body_handle = ps.rigid_body_set.insert(body);

        let collider_handle = ps.collider_set.insert_with_parent(
            collider,
            body_handle,
            &mut ps.rigid_body_set,
        );

        // calculating the jump height based on mass
        {
            let body = ps.rigid_body_set.get_mut(body_handle).unwrap();

            if let Some(jump_height) = self.jump_heights.get_mut(parent_id) {
                let initial_velocity = (2.0 * GRAVITY.abs() * jump_height.desired).sqrt();
                let impulse = glam::vec3(0.0, body.mass() * initial_velocity, 0.0);

                jump_height.precalculated = Some(impulse.into()); 
            }
        }

        let physics_handle = PhysicsHandle {
            rigid_body: body_handle,
            collider: collider_handle,

            og_rb_type: RigidBodyType::Dynamic,
        };

        self.physics_handles.insert(parent_id, physics_handle);

        

        let collider_model = Pill {
            r,
            h: capsule_total_height,
        }.create_model(12, 5, offset);

        self.collider_transforms.insert(parent_id, Transform {
            position: pill_pos,
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(1.0),
        });
        self.prev_collider_transforms.insert(parent_id, Transform {
            position: pill_pos,
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(1.0),
        });
        self.collider_gizmos.insert(parent_id, collider_model);

        self.grounded_states.insert(parent_id, GroundedState {
            was_grouned: false,
            is_grounded: false,
            just_left: false,
            just_landed: false,
            ray_length_grounded: 0.25,
            ray_length_airborn: 0.06,
        });

        self.collider_to_entity.insert(collider_handle, parent_id);
    }

    pub fn create_cylinder_hitbox(
        &mut self, 
        r: f32,
        h: f32,
        position: Vec3,
        scale: Vec3,
        rotation: Quat,
        parent_id: usize,
        ps: &mut PhysicsState,
    ) {
        let cyl = Cylinder {
            r,
            h,
        };

        let cyl_mod = cyl.create_model(10);

        self.collider_gizmos.insert(parent_id, cyl_mod);
        self.collider_transforms.insert(parent_id, Transform {
            position,
            rotation: Quat::IDENTITY,
            scale,
        });
        self.prev_collider_transforms.insert(parent_id, Transform {
            position,
            rotation: Quat::IDENTITY,
            scale,
        });

        let iso: Isometry<f32> = (position, rotation).into();

        let body = RigidBodyBuilder::fixed()
            .position(iso)
            .build();


        let collider = ColliderBuilder::cylinder(h * 0.5, r)
            .active_collision_types(ActiveCollisionTypes::all())
            .build();

        let body_handle = ps.rigid_body_set.insert(body);
        let collider_handle = ps.collider_set.insert_with_parent(
            collider,
            body_handle,
            &mut ps.rigid_body_set,
        );
        
        // set body massA

        self.physics_handles.insert(parent_id, PhysicsHandle {
            rigid_body: body_handle,
            collider: collider_handle,

            og_rb_type: RigidBodyType::Fixed,
        });

        self.collider_to_entity.insert(collider_handle, parent_id);
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

        self.collider_gizmos.insert(parent_id, cuboid_model);
        self.model_heights.insert(parent_id, size.y);


        self.collider_transforms.insert(parent_id, Transform {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale,
        });
        self.prev_collider_transforms.insert(parent_id, Transform {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale,
        });

        // PHYSICS PASS
        let iso: Isometry<f32> = (position, rotation).into();

        let body = RigidBodyBuilder::kinematic_position_based()
            .position(iso)
            .ccd_enabled(true)
            .build();

        let half_extents = size * 0.5;

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

            og_rb_type: RigidBodyType::KinematicPositionBased,
        });

        self.collider_to_entity.insert(collider_handle, parent_id);
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

        // Process vertices into arrays
        let vertices: Vec<Point3<f32>> = model.vertices
            .iter()
            .map(|v| v.position.into())
            .collect();
        
        let indices: Vec<[u32; 3]> = model.indices
            .chunks(3)
            .map(|chunk| [chunk[0], chunk[1], chunk[2]])
            .collect();

        self.collider_transforms.insert(parent_id, Transform {
            position,
            rotation: Quat::IDENTITY,
            scale,
        });
        self.prev_collider_transforms.insert(parent_id, Transform {
            position,
            rotation: Quat::IDENTITY,
            scale,
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

        self.physics_handles.insert(parent_id, PhysicsHandle {
            rigid_body: body_handle,
            collider: collider_handle,

            og_rb_type: RigidBodyType::Fixed,
        });

        self.collider_to_entity.insert(collider_handle, parent_id);
    }

    pub fn update(&mut self, sm: &mut SoundManager, ps: &mut PhysicsState, input: &mut InputState) {
        if input.just_pressed(glfw::Key::Delete) {
            for i in self.selected.iter() {
                self.entity_trashcan.push(*i);
            }
        }
        self.delete_entities(sm, ps);
    }

    pub fn delete_entities(&mut self, sm: &mut SoundManager, ps: &mut PhysicsState) {
        for id in &self.entity_trashcan {
            if let Some(ph) = self.physics_handles.get_mut(*id) {
                self.collider_to_entity.remove(&ph.collider);
                ps.rigid_body_set.remove(
                    ph.rigid_body,
                    &mut ps.island_manager,
                    &mut ps.collider_set,
                    &mut ps.impulse_joint_set,
                    &mut ps.multibody_joint_set,
                    true,
                );
            }

            self.transforms.remove(*id);
            self.prev_transforms.remove(*id);
            self.local_corrections.remove(*id);
            self.collider_gizmos.remove(*id);
            self.collider_transforms.remove(*id);
            self.prev_collider_transforms.remove(*id);
            
            // remove the ownership relation from the inventory item_bones
            if let Some(inv) = self.inventories.get(*id) {
                for i in inv.iter() {
                    self.owners.remove(*i);
                }
            }
            self.inventories.remove(*id);

            // find the active weapon to drop
            if let Some(ai) = self.active_items.get(*id) {
                match ai.right_hand {
                    Some(rhid) => {
                        self.owners.remove(rhid);
                        self.is_equipped.remove(rhid);
                    }
                    None => (),
                }
                match ai.left_hand {
                    Some(lhid) => {
                        self.owners.remove(lhid);
                        self.is_equipped.remove(lhid);
                    }
                    None => (),
                }
            }
            self.active_items.remove(*id);

            self.owners.remove(*id);
            
            // find the equipped flags to drop
            self.is_equipped.remove(*id);

            self.factions.remove(*id);
            self.entity_types.remove(*id);
            self.models.remove(*id);
            self.animators.remove(*id);
            self.skellingtons.remove(*id);
            self.rotators.remove(*id);
            self.impulse_applied.remove(*id);
            self.player_controllers.remove(*id);
            self.simstate_controllers.remove(*id);
            self.destinations.remove(*id);
            self.v_effects.remove(*id);
            self.physics_handles.remove(*id);
            self.hitsets.remove(*id);
            self.yaws.remove(*id);
            self.knockbacks.remove(*id);
            self.healths.remove(*id);
            self.base_speeds.remove(*id);
            self.aggro_ranges.remove(*id);
            self.jump_heights.remove(*id);
            self.total_masses.remove(*id);
            self.model_heights.remove(*id);
            self.grounded_states.remove(*id);
        }

        self.entity_trashcan.clear();
    }

    pub fn get_ids_for_faction(&self, faction: &str) -> Vec<usize> {
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

    pub fn get_ids_for_type(&self, entity_type: &str) -> Vec<usize> {
        let result: Vec<usize> = self.entity_types
            .iter()
            .filter_map(|f|
                if f.value() == entity_type {
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

    pub fn get_all_orphaned_weapon_ids(&self) -> Vec<usize> {
        self.factions
            .iter()
            .filter(|w_type| {
                *w_type.value() == "Item"
                && self.owners.get(w_type.key()).is_none()
            })
            .map(|e| e.key())
            .collect::<Vec<usize>>()
    }

    pub fn get_all_unequipped_owned_ids(&self) -> Vec<usize> {
        self.factions
            .iter()
            .filter(|w_type| {
                *w_type.value() == "Item"
                && self.owners.get(w_type.key()).is_some()
                && self.is_equipped.get(w_type.key()).is_none()
            })
            .map(|e| e.key())
            .collect::<Vec<usize>>()
    }

    pub fn get_active_weapon_ids(&self) -> Vec<usize> {
        self.is_equipped
            .iter()
            .map(|e| e.key())
            .collect::<Vec<usize>>()
    }

    pub fn get_non_weapon_entities(&self) -> Vec<usize> {
        self.factions
            .iter()
            .filter(|w_type| {
                *w_type.value() != "Item"
                //&& *w_type.value() != Faction::World
            })
            .map(|e| e.key())
            .collect::<Vec<usize>>()
    }

    pub fn get_gizmo_ids(&self) -> Vec<usize> {
        self.collider_gizmos
            .iter()
            .map(|e| e.key())
            .collect::<Vec<usize>>()
    }

    pub fn empty_selected_and_reset_bodies(&mut self, ps: &mut PhysicsState) {
        // TODO: We could create a struct that contains the rb handle and the entity
        for id in self.selected.iter() {
            if let Some(ph) = self.physics_handles.get(*id) {
                let rb = ps.rigid_body_set.get_mut(ph.rigid_body).unwrap();

                rb.set_body_type(ph.og_rb_type, false);
            }
        }

        self.selected.clear();
    }

    pub fn serialize_entity_data(&self) {
        let mut wd = WorldData { entities: vec![] };

        for etype in self.entity_types.iter() {
            if etype.value() == "Terrain" { continue; }

            let id = etype.key();

            match self.owners.get(id) {
                Some(_) => continue,
                None => (),
            }

            let weapons = self.resolve_weapons(id);

            let faction = self.factions.get(id).unwrap().clone();

            let jump_height = match self.jump_heights.get(id) {
                Some(jh) => Some(jh.desired),
                _=> None,
            };
            let instance = EntityInstance {
                entity_type: etype.value().clone(),
                faction,
                position: self.transforms.get(id).unwrap().position,
                rotation: self.transforms.get(id).unwrap().rotation,
                weapons,
                base_speed: self.base_speeds.get(id).copied(),
                jump_height,
                health: self.healths.get(id).copied(),
            };

            wd.entities.push(instance);
        }

        wd.write_to_file("config/world_data.json");
    }

    pub fn resolve_weapons(&self, id: usize) -> Option<Vec<EntityInstance>> {
        if let Some(inv) = self.inventories.get(id) {
            let mut idlist = inv.clone();
            let mut wlist = vec![];

            if let Some(aa) = self.active_items.get(id) {
                if let Some(lh) = aa.left_hand {
                    idlist.push(lh);
                }

                if let Some(rh) = aa.right_hand {
                    idlist.push(rh);
                }
            }

            for w in idlist.iter() {
                wlist.push(EntityInstance {
                    entity_type: self.entity_types.get(*w).unwrap().clone(),
                    faction: self.factions.get(*w).unwrap().clone(),
                    position: Vec3::splat(0.0),
                    rotation: Quat::IDENTITY,
                    weapons: None,
                    base_speed: None, 
                    jump_height: None,
                    health: None,
                });
            }

            return Some(wlist);
        }
        None
    }
}

pub fn glam_to_nalgebra_quat(q: Quat) -> UnitQuaternion<f32> {
    UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(q.w, q.x, q.y, q.z))
}

pub fn load_terrain(entity_manager: &mut EntityManager, physics_state: &mut PhysicsState) {
    //let path = "resources/textures/brushes/301B1.png";
    //let path = "resources/textures/brushes/testing.png";
    //let path = "resources/textures/brushes/mountain.png";
    let path = "resources/textures/brushes/blendertest.png";
    let img = image::open(path).expect("Failed to load terrain image").to_luma8();
    let (width, height) = img.dimensions();
    let y_amplitude = 25.0;
    let mut terrain = Terrain::from_height_map(y_amplitude, width, height, &img);
    //let mut terrain = Terrain::from_height_map("resources/textures/solid-black-100-100.png");
    //let mut terrain = Terrain::from_height_map("resources/textures/brushes/NvF5e.jpg");
    //let mut terrain = Terrain::from_height_map("resources/textures/brushes/big_spot.jpeg");
    //let mut terrain = Terrain::from_height_map("resources/textures/brushes/2000.png");

    let model = terrain.into_opengl_model();

    let terrain_trans = Transform {
        position: Vec3::splat(0.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::splat(1.0),
    };

    entity_manager.transforms.insert(entity_manager.next_entity_id, terrain_trans.clone(), );
    //TODO: load this dynamically potentially
    entity_manager.factions.insert(entity_manager.next_entity_id, "World".to_string());
    entity_manager.faction_register.insert("World".to_string());
    entity_manager.entity_types.insert(entity_manager.next_entity_id, "Terrain".to_string());

    entity_manager.collider_transforms.insert(entity_manager.next_entity_id, terrain_trans.clone());

    let iso: Isometry<f32> = (terrain_trans.position, terrain_trans.rotation).into();
    let body = RigidBodyBuilder::fixed().position(iso)
        .build();

    // Process vertices into arrays
    //let vertices: Vec<Point3<f32>> = model.vertices
    //    .iter()
    //    .map(|v| v.position.into())
    //    .collect();
    //
    //let indices: Vec<[u32; 3]> = model.indices
    //    .chunks(3)
    //    .map(|chunk| [chunk[0], chunk[1], chunk[2]])
    //    .collect();

    //let (heights, nrows, ncols) = Terrain::heights_from_image(y_amplitude, &img, width, height);


    //let terrain_collider = ColliderBuilder::trimesh(vertices, indices).unwrap();
    //let terrain_collider = ColliderBuilder::trimesh(vertices, indices).unwrap();
    //let terrain_collider = ColliderBuilder::heightfield(
    //    heights, 
    //    Vector3::new((ncols - 1) as f32, 1.0, (nrows - 1) as f32)
    //).build();
    // let terrain_collider = ColliderBuilder::cuboid(50.0, 0.5, 50.0).build();

    let body_handle = physics_state.rigid_body_set.insert(body);
    //let collider_handle = physics_state.collider_set.insert_with_parent(
    //    terrain_collider,
    //    body_handle,
    //    &mut physics_state.rigid_body_set,
    //);
    terrain::insert_chunked_terrain_colliders(
        &model,
        width, height,
        128, 128, // how big is each chunk
        body_handle,
        &mut physics_state.collider_set,
        &mut physics_state.rigid_body_set,
    );

    //entity_manager.physics_handles.insert(entity_manager.next_entity_id, PhysicsHandle {
    //    rigid_body: body_handle,
    //    collider: collider_handle,
    //});

    entity_manager.models.insert(entity_manager.next_entity_id, model);

    entity_manager.next_entity_id += 1;
}
