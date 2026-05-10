#![allow(unused_must_use)]
use core::f32;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::UNIX_EPOCH,
};

use glam::{vec3, Quat, Vec3};
use nalgebra::{Point3, UnitQuaternion};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rapier3d::prelude::*;
use winit::keyboard::KeyCode;

use crate::{
    abilities::{AbilitiesConfig, WeaponAbilities, WeaponPoolsConfig},
    animation::{
        self,
        animation::Animation,
        animator::{Animator, RootMotionState},
        model::Model,
        skellington::Bone,
    },
    assets,
    command_buffer::{CommandBuffer, EcsAction, ImpulseKind},
    config::{
        entity_config::{
            AnimationPropHelper, EntityConfig, EntityTypeHelper, ItemBones, UiEntityTypeHelper,
        },
        factions_config::FactionsConfig,
        weapon_anim_map::{WeaponActionsHelper, WeaponAnimMapHelper},
        world_data::{EntityInstance, WorldData},
        Config,
    },
    debug::gizmos::{Cuboid, Cylinder, Dimension, Pill, Sphere},
    enums_types::{
        ActiveItem, AnimationType, ControlState, Counter, DamageSource, DamageVolume,
        DamageVolumeHelper, EnemyController, FrameActivation, GroundedState, HitboxShape,
        JumpHeight, Knockback, LifeState, LocoState, PhysicsHandle, PlayerController, Rotator,
        StatusEffect, Transform, VisualEffect,
    },
    input::InputState,
    physics::PhysicsState,
    projectile_system::ProjectileController,
    sound::sound_manager::{ContinuousSound, OneShot, SoundManager},
    sparse_set::SparseSet,
    state_machines::enemy::enemy_behavior_tree::{self, BehaviorTree},
    terrain::{self, Terrain},
    util::constants::{GRAVITY, GROUP_PLAYER},
};

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

    pub world_weapon_tips: SparseSet<Vec3>,

    // Mostly for "areas" that are gizmo-only entities.
    pub dimensions: SparseSet<Dimension>,

    pub inventories: SparseSet<Vec<usize>>,
    pub active_items: SparseSet<ActiveItem>,

    // This is for a weapon to "know" that it is in an inventory.
    // LHS: weapon_id, RHS: player_id
    // LHS: damage_volume_id, RHS: player_id
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
    pub enemy_controllers: SparseSet<EnemyController>,
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
    pub max_healths: SparseSet<f32>,
    pub manas: SparseSet<f32>,
    pub max_manas: SparseSet<f32>,
    pub levels: SparseSet<u32>,
    pub names: SparseSet<String>,
    pub base_speeds: SparseSet<f32>,
    pub aggro_ranges: SparseSet<f32>,
    pub jump_heights: SparseSet<JumpHeight>,
    pub total_masses: SparseSet<f32>,
    pub model_heights: SparseSet<f32>,
    pub grounded_states: SparseSet<GroundedState>,
    pub cleanup_timer: SparseSet<f32>,
    pub pickup_ranges: SparseSet<f32>,
    pub weapon_helper: SparseSet<WeaponActionsHelper>,
    pub behavior_trees: SparseSet<BehaviorTree>,
    pub projectile_controllers: SparseSet<ProjectileController>,
    pub damage_volumes: SparseSet<DamageVolume>,
    pub status_effects: SparseSet<Vec<StatusEffect>>,

    // Everything below here feels like a different thing than the ECS. resources?

    // Projectile stuff
    // Source id is the ID of the character who originally spawned this projectile (such as a
    // wizard casting a fireball, etc.)
    pub source_ids: SparseSet<usize>,
    // how long the projectile lasts before dying.
    pub lifetimes: SparseSet<Counter>,

    /// Abilities assigned to weapon entities.
    pub weapon_abilities: SparseSet<WeaponAbilities>,

    pub entity_type_register: HashMap<String, EntityTypeHelper>,
    pub faction_register: HashSet<String>,

    /// Ability definitions (loaded from config).
    pub abilities_config: AbilitiesConfig,
    /// Weapon ability pool configuration.
    pub weapon_pools_config: WeaponPoolsConfig,

    pub serializable_world_data: WorldData,
    pub weapon_anim_map: WeaponAnimMapHelper,

    pub current_round_enemies: Vec<usize>,
    pub animation_to_damage_volume: HashMap<AnimationType, DamageVolumeHelper>,
}

impl EntityManager {
    pub fn new(max_entities: usize) -> Self {
        let wd = WorldData::load_from_file("config/world_data.json");

        let mut weapon_anim_map =
            WeaponAnimMapHelper::load_or_create_default("config/weapon_anim_map.json");

        for helper in weapon_anim_map.weapon_types.values_mut() {
            if helper.basic_chain_default.is_empty() {
                helper.basic_chain_default = helper.basic_chain.clone();
            }
        }

        Self {
            next_entity_id: 0,
            transforms: SparseSet::with_capacity(max_entities),
            prev_transforms: SparseSet::with_capacity(max_entities),
            local_corrections: SparseSet::with_capacity(max_entities),
            collider_gizmos: SparseSet::with_capacity(max_entities),
            collider_transforms: SparseSet::with_capacity(max_entities),
            prev_collider_transforms: SparseSet::with_capacity(max_entities),
            collider_to_entity: HashMap::new(),
            world_weapon_tips: SparseSet::with_capacity(max_entities),
            dimensions: SparseSet::with_capacity(max_entities),
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
            enemy_controllers: SparseSet::with_capacity(max_entities),
            destinations: SparseSet::with_capacity(max_entities),
            rng: ChaCha8Rng::seed_from_u64(100),
            selected: Vec::new(),
            v_effects: SparseSet::with_capacity(max_entities),
            entity_trashcan: Vec::new(),
            physics_handles: SparseSet::with_capacity(max_entities),
            hitsets: SparseSet::with_capacity(max_entities),
            yaws: SparseSet::with_capacity(max_entities),
            knockbacks: SparseSet::with_capacity(max_entities),
            healths: SparseSet::with_capacity(max_entities),
            max_healths: SparseSet::with_capacity(max_entities),
            manas: SparseSet::with_capacity(max_entities),
            max_manas: SparseSet::with_capacity(max_entities),
            levels: SparseSet::with_capacity(max_entities),
            names: SparseSet::with_capacity(max_entities),
            base_speeds: SparseSet::with_capacity(max_entities),
            aggro_ranges: SparseSet::with_capacity(max_entities),
            jump_heights: SparseSet::with_capacity(max_entities),
            total_masses: SparseSet::with_capacity(max_entities),
            model_heights: SparseSet::with_capacity(max_entities),
            grounded_states: SparseSet::with_capacity(max_entities),
            cleanup_timer: SparseSet::with_capacity(max_entities),
            pickup_ranges: SparseSet::with_capacity(max_entities),
            source_ids: SparseSet::with_capacity(max_entities),
            lifetimes: SparseSet::with_capacity(max_entities),
            weapon_abilities: SparseSet::with_capacity(max_entities),
            weapon_helper: SparseSet::with_capacity(max_entities),
            behavior_trees: SparseSet::with_capacity(max_entities),
            projectile_controllers: SparseSet::with_capacity(max_entities),
            damage_volumes: SparseSet::with_capacity(max_entities),
            status_effects: SparseSet::with_capacity(max_entities),

            // TODO: Probably just return the entity_types here instead of accessing them like this
            entity_type_register: EntityConfig::load_from_file("config/entity_config.json")
                .entity_types,
            faction_register: FactionsConfig::load_from_file("config/factions_config.json")
                .factions,
            abilities_config: AbilitiesConfig::load_or_create_default(
                "config/abilities_config.json",
            ),
            weapon_pools_config: WeaponPoolsConfig::load_or_create_default(
                "config/weapon_pools_config.json",
            ),
            serializable_world_data: wd,
            weapon_anim_map,
            current_round_enemies: Vec::new(),
            animation_to_damage_volume: HashMap::new(),
        }
    }

    pub fn populate_entity_data(&mut self, ps: &mut PhysicsState) {
        let data = self.serializable_world_data.clone();

        for instance in data.entities.iter() {
            if let Some(et) = self.entity_type_register.get(&instance.entity_type) {
                match et.mesh_path.as_str() {
                    "" => {
                        // meshless things are gizmo-only things such as a spawn area or a trigger
                        // area. There has to be a gizmo, it can't be a null entity type, that is
                        // not supported, yet
                        self.create_meshless_entity(instance);
                    }
                    _ => {
                        let parent_id = self.create_mesh_entity(instance, ps);
                        self.populate_inventory(parent_id, &instance, ps);
                    }
                }
            }
        }

        load_terrain(self, ps);
    }

    pub fn create_weapon(&mut self, instance: &EntityInstance, ps: &mut PhysicsState) -> usize {
        let weapon_id = self.create_mesh_entity(instance, ps);
        self.hitsets.insert(weapon_id, HashSet::new());

        let abilities = WeaponAbilities::generate(
            &instance.entity_type,
            &self.weapon_pools_config,
            &mut self.rng,
        );
        self.weapon_abilities.insert(weapon_id, abilities);

        self.weapon_helper.insert(
            weapon_id,
            self.weapon_anim_map
                .weapon_types
                .get(&instance.entity_type)
                .unwrap()
                .clone(),
        );

        weapon_id
    }

    pub fn populate_inventory(
        &mut self,
        parent_id: usize,
        instance: &EntityInstance,
        ps: &mut PhysicsState,
    ) {
        if let Some(weapons_list) = &instance.weapons {
            for weapon in weapons_list.iter() {
                let weapon_id = self.create_weapon(weapon, ps);

                match self.entity_types.get(weapon_id).unwrap().as_str() {
                    "OrcSword" | "DoubleAxe" => {
                        let bt = BehaviorTree::new("melee");
                        self.behavior_trees.insert(parent_id, bt);
                    }
                    "FireStaff" | "IceStaff" | "Staff" => {
                        let bt = BehaviorTree::new("ranged");
                        self.behavior_trees.insert(parent_id, bt);
                    }
                    _ => panic!("don't do that"),
                }

                self.owners.insert(weapon_id, parent_id);

                match self.inventories.get_mut(parent_id) {
                    Some(inv) => {
                        if !inv.contains(&weapon_id) {
                            inv.push(weapon_id);
                        }
                    }
                    None => {
                        let inv = vec![weapon_id];
                        self.inventories.insert(parent_id, inv);
                    }
                }

                match self.active_items.get_mut(parent_id) {
                    Some(_) => (),
                    None => {
                        self.active_items.insert(
                            parent_id,
                            ActiveItem {
                                right_hand: Some(weapon_id),
                                left_hand: None,
                            },
                        );

                        if let Some(inv) = self.inventories.get_mut(parent_id) {
                            inv.retain(|v| *v != weapon_id);
                        }

                        self.is_equipped.insert(weapon_id, true);
                    }
                }
            }
        }
    }

    // right now this is just for things that might be "gizmo-only" such as a spawn area
    pub fn create_meshless_entity(&mut self, instance: &EntityInstance) {
        let parent_id = self.next_entity_id;

        dbg!("CREATING A MESHLESS ENTITY");

        let archetype = match self.entity_type_register.get(&instance.entity_type) {
            Some(a) => a,
            None => {
                dbg!(&instance.entity_type);
                panic!();
            }
        };

        let position = instance.position;
        let rotation = instance.rotation;
        let scale = archetype.scale_correction;

        self.entity_types
            .insert(parent_id, instance.entity_type.clone());

        match archetype.hitbox {
            HitboxShape::Cylinder { r, h } => {
                self.create_cylinder_hitbox_no_physics(r, h, position, scale, parent_id);
                self.dimensions
                    .insert(parent_id, Dimension::Cylinder { r, h });
            }
            HitboxShape::BoxDim {
                hx: _hx,
                hy: _hy,
                hz: _hz,
            } => {
                todo!();
            }
            _ => panic!("don't you try it you swit!!!"),
        }

        self.transforms.insert(
            parent_id,
            Transform {
                position,
                rotation,
                scale,
            },
        );

        self.next_entity_id += 1;
    }

    // source_id is the character that spawned it
    pub fn create_damage_volume(
        &mut self,
        source_id: usize,
        anim: &AnimationType,
        ps: &mut PhysicsState,
    ) {
        let volume_id = self.next_entity_id;
        self.next_entity_id += 1;

        let volume_conf = self
            .animation_to_damage_volume
            .get_mut(anim)
            .cloned()
            .unwrap();

        let volume = DamageVolume {
            source_anim: *anim,
            source: DamageSource::Entity(source_id),
            shape: volume_conf.shape,
            ticker: volume_conf.ticker,
            offset: volume_conf.offset,
            damage_payload: volume_conf.damage_payload,
        };

        let source_transform = self.transforms.get(source_id).unwrap();
        let source_position = source_transform.position;
        let source_rotation = -source_transform.rotation;

        self.transforms.insert(
            volume_id,
            Transform {
                position: source_position,
                rotation: source_rotation,
                scale: Vec3::ONE,
            },
        );

        match volume.shape {
            HitboxShape::Cylinder { r, h } => {
                self.create_cylinder_hitbox(
                    r,
                    h,
                    source_position,
                    Vec3::ONE,
                    source_rotation,
                    volume_id,
                    volume.offset,
                    ps,
                );
            }
            _ => eprint!("invalid shape passed for hitbox"),
        }

        // physics

        self.damage_volumes.insert(volume_id, volume);
        self.hitsets.insert(volume_id, HashSet::new());
    }

    pub fn create_sphere_projectile_from_weapon(
        &mut self,
        source_id: usize,
        weapon_id: usize,
        ps: &mut PhysicsState,
    ) -> Option<usize> {
        self.next_entity_id += 1;
        let projectile_id = self.next_entity_id;

        let Some(origin) = self.world_weapon_tips.get(weapon_id) else {
            eprintln!("Failed to find the tip for the given weapon");
            return None;
        };

        self.source_ids.insert(projectile_id, source_id);
        self.lifetimes.insert(
            projectile_id,
            Counter {
                ttl: 0.5,
                accumulator: 0.0,
            },
        );

        let mut instance =
            EntityInstance::new("SphereProjectile".to_string(), *origin, Quat::IDENTITY);
        instance.faction = Some("Projectile".to_string());

        Some(self.create_mesh_entity(&instance, ps))
    }

    pub fn create_mesh_entity(
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
        let scale = archetype.scale_correction;

        match &instance.faction {
            Some(faction) => {
                self.factions.insert(parent_id, faction.clone());
                self.faction_register.insert(faction.clone());

                match faction.as_str() {
                    "Player" => {
                        self.player_controllers
                            .insert(parent_id, PlayerController::default());

                        // default player pickup range
                        self.pickup_ranges.insert(parent_id, 3.0);

                        self.projectile_controllers
                            .insert(parent_id, ProjectileController::new());
                    }
                    "Enemy" => {
                        self.destinations.insert(parent_id, position);

                        self.enemy_controllers
                            .insert(parent_id, EnemyController::default());

                        self.projectile_controllers
                            .insert(parent_id, ProjectileController::new());

                        if let Some(ar) = archetype.aggro_range {
                            self.aggro_ranges.insert(parent_id, ar);
                        }
                    }
                    _ => (),
                }
            }
            None => {
                // TODO: Determine whether this is true, for now let's catch these while we have no
                // specific use-case for them yet. Factionless entities should probably go the
                // meshelss path
                panic!("I don't know if we should have a meshed entity without a faction");
            }
        }

        self.yaws.insert(parent_id, 0.0);

        if let Some(health) = instance.health {
            self.healths.insert(parent_id, health);
        }

        if let Some(max_health) = instance.max_health {
            self.max_healths.insert(parent_id, max_health);
        }

        if let Some(mana) = instance.mana {
            self.manas.insert(parent_id, mana);
        }

        if let Some(max_mana) = instance.max_mana {
            self.max_manas.insert(parent_id, max_mana);
        }

        if let Some(level) = instance.level {
            self.levels.insert(parent_id, level);
        }

        if let Some(pr) = instance.pickup_range {
            self.pickup_ranges.insert(parent_id, pr);
        } else if let Some(pr) = archetype.pickup_range {
            self.pickup_ranges.insert(parent_id, pr);
        }

        if let Some(name) = &instance.name {
            self.names.insert(parent_id, name.clone());
        }

        if let Some(base_speed) = instance.base_speed {
            self.base_speeds.insert(parent_id, base_speed);
        }

        if let Some(jump_height) = instance.jump_height {
            self.jump_heights.insert(
                parent_id,
                JumpHeight {
                    desired: jump_height,
                    precalculated: None,
                },
            );
        }

        if let Some(total_mass) = archetype.total_mass {
            self.total_masses.insert(parent_id, total_mass);
        }

        if let Some(cleanup) = instance.cleanup_timer {
            self.cleanup_timer.insert(parent_id, cleanup);
        }

        let transform = Transform {
            position,
            rotation,
            scale,
        };
        self.transforms.insert(parent_id, transform.clone());

        // CHECK FOR BONES
        let model = if let (Some(bone_path), Some(anim_props)) =
            (&archetype.bone_path, &archetype.animation_properties)
        {
            //let (skellington, mut animator, animation) =
            //animation::animation::import_bone_data(bone_path, false);
            let (skellington, mut animator, animation) = if let Some(already_type) = self
                .entity_types
                .iter()
                .find(|e| *e.value() == instance.entity_type)
                .map(|e| e.key())
            {
                let skell = self.skellingtons.get(already_type).unwrap().clone();
                let animator = self.animators.get(already_type).unwrap().clone();
                let animation = animator.animations.iter().next().unwrap().1.clone();

                if let Some(ib) = self.item_bones.get(already_type).cloned() {
                    self.item_bones.insert(parent_id, ib);
                }

                (skell, animator, animation)
            } else {
                let b = if let Some(b) = &archetype.item_bones {
                    Some(b.rh.as_str())
                } else {
                    None
                };

                let (skell, mut animator, animation, rh_bone_id) =
                    animation::data_loader::import_bone_data(bone_path, false, b);

                if let Some(_) = &archetype.item_bones {
                    if rh_bone_id.is_some() {
                        self.item_bones.insert(
                            parent_id,
                            ItemBones {
                                rh: rh_bone_id.unwrap(),
                                lh: 0,
                            },
                        );
                    }
                }

                if let Some(bone) = &archetype.root_bone {
                    animator.root_motion_state = RootMotionState {
                        root_bone: bone.clone(),
                        last_root_pos: None,
                        frame_root_delta: Vec3::ZERO,
                        active_source: None,
                    };
                }

                (skell, animator, animation)
            };

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

                    if let Some(range_list) = &prop.hurtbox_activation {
                        let mut list = vec![];

                        for r in range_list {
                            list.push(FrameActivation {
                                segment_range: r[0]..=r[1],
                                triggered: false.into(),
                            });
                        }

                        anim.hurtbox_activation = Some(list);
                    }

                    anim.hold_frame = prop.hold_frame;
                    anim.interrupt_frame = prop.interrupt_frame;
                    anim.reset_on_change = prop.reset_on_change;
                    anim.do_root_motion = prop.do_root_motion;
                    anim.projectile_frame = prop.projectile_frame;
                }

                if let Some(dv) = &prop.damage_volume {
                    self.animation_to_damage_volume
                        .insert(prop.name, dv.clone());
                }
            }

            let model = self
                .models
                .iter()
                .find(|m| m.value().full_path == *archetype.mesh_path)
                .map(|m| m.value().clone())
                .unwrap_or_else(|| {
                    animation::data_loader::import_model_data(&archetype.mesh_path, &animation)
                });

            let rotator = Rotator {
                cur_rot: instance.rotation,
                next_rot: instance.rotation,
                blend_factor: 0.0,
                blend_time: 0.11,
            };

            animator.set_next_animation(AnimationType::Idle);

            self.animators.insert(parent_id, animator);
            self.skellingtons.insert(parent_id, skellington);
            self.models.insert(parent_id, model.clone());
            self.rotators.insert(parent_id, rotator);

            model
        } else {
            let model = self
                .models
                .iter()
                .find(|m| m.value().full_path == *archetype.mesh_path)
                .map(|m| m.value().clone())
                .unwrap_or_else(|| {
                    animation::data_loader::import_model_data(
                        &archetype.mesh_path,
                        &Animation::default(),
                    )
                });

            self.models.insert(parent_id, model.clone());

            model
        };

        self.entity_types
            .insert(parent_id, instance.entity_type.clone());

        self.prev_transforms.insert(parent_id, transform);
        self.entity_types
            .insert(parent_id, instance.entity_type.clone());

        self.local_corrections.insert(
            parent_id,
            Transform {
                position: glam::Vec3::splat(0.0),
                scale: glam::Vec3::splat(1.0),
                rotation: archetype.rot_correction,
            },
        );

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
                    Vec3::ZERO,
                    ps,
                );
            }
            HitboxShape::Pill { r, h } => {
                self.create_pill_hitbox(
                    r,
                    h,
                    position,
                    scale,
                    rotation,
                    parent_id,
                    ps,
                    archetype.rigid_body_type,
                );
            }
            HitboxShape::BoundingBox => {
                self.create_bounding_hitbox(&model, position, scale, rotation, parent_id, ps);
            }
            HitboxShape::Mesh => {
                self.create_mesh_based_hitbox(&model, position, scale, rotation, parent_id, ps);
            }
            HitboxShape::Sphere { r } => {
                self.create_sphere_hitbox(r, position, scale, parent_id, ps)
            }
            _ => (),
        }

        parent_id
    }

    pub fn create_pill_hitbox(
        &mut self,
        r: f32,
        h: f32,
        position: Vec3,
        _scale: Vec3,
        rotation: Quat,
        parent_id: usize,
        ps: &mut PhysicsState,
        rbt: Option<RigidBodyType>,
    ) {
        let pill_pos = position;
        // === PHYSICS ===
        let iso: Isometry<f32> = (pill_pos, rotation).into();

        let body = match rbt {
            Some(rbt) => match rbt {
                RigidBodyType::KinematicPositionBased => {
                    RigidBodyBuilder::kinematic_position_based()
                        .ccd_enabled(false)
                        .position(iso)
                        .enabled_rotations(true, true, true)
                        .build()
                } // TODO: Handle all types
                _ => RigidBodyBuilder::dynamic()
                    .ccd_enabled(false)
                    .position(iso)
                    .enabled_rotations(false, false, false)
                    .build(),
            },
            None => RigidBodyBuilder::dynamic()
                .ccd_enabled(false)
                .position(iso)
                .enabled_rotations(false, false, false)
                .build(),
        };

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

        let collider_handle =
            ps.collider_set
                .insert_with_parent(collider, body_handle, &mut ps.rigid_body_set);

        // calculating the jump height based on mass
        let physics_handle = {
            let body = ps.rigid_body_set.get_mut(body_handle).unwrap();

            if let Some(jump_height) = self.jump_heights.get_mut(parent_id) {
                let initial_velocity = (2.0 * GRAVITY.abs() * jump_height.desired).sqrt();
                let impulse = glam::vec3(0.0, body.mass() * initial_velocity, 0.0);

                jump_height.precalculated = Some(impulse.into());
            }

            PhysicsHandle {
                rigid_body: body_handle,
                collider: collider_handle,

                og_rb_type: body.body_type(),
            }
        };

        self.physics_handles.insert(parent_id, physics_handle);

        let collider_model = Pill {
            r,
            h: capsule_total_height,
        }
        .create_model(12, 5, offset);

        self.collider_transforms.insert(
            parent_id,
            Transform {
                position: pill_pos,
                rotation: Quat::IDENTITY,
                scale: Vec3::splat(1.0),
            },
        );
        self.prev_collider_transforms.insert(
            parent_id,
            Transform {
                position: pill_pos,
                rotation: Quat::IDENTITY,
                scale: Vec3::splat(1.0),
            },
        );
        self.collider_gizmos.insert(parent_id, collider_model);

        self.grounded_states.insert(
            parent_id,
            GroundedState {
                was_grounded: false,
                is_grounded: false,
                just_left: false,
                just_landed: false,
                ray_length_grounded: 0.25,
                ray_length_airborne: 0.06,
            },
        );

        self.collider_to_entity.insert(collider_handle, parent_id);
    }

    pub fn create_cylinder_hitbox_no_physics(
        &mut self,
        r: f32,
        h: f32,
        position: Vec3,
        scale: Vec3,
        parent_id: usize,
    ) {
        let cyl = Cylinder { r, h };

        let cyl_mod = cyl.create_model(10);

        self.collider_gizmos.insert(parent_id, cyl_mod);
        self.collider_transforms.insert(
            parent_id,
            Transform {
                position,
                rotation: Quat::IDENTITY,
                scale,
            },
        );
        self.prev_collider_transforms.insert(
            parent_id,
            Transform {
                position,
                rotation: Quat::IDENTITY,
                scale,
            },
        );
    }

    pub fn create_cylinder_hitbox(
        &mut self,
        r: f32,
        h: f32,
        position: Vec3,
        _scale: Vec3,
        rotation: Quat,
        parent_id: usize,
        offset: Vec3,
        ps: &mut PhysicsState,
    ) {
        let cyl_pos = position;

        // === PHYSICS ===
        let iso: Isometry<f32> = (cyl_pos, rotation).into();

        let body = RigidBodyBuilder::kinematic_position_based()
            .ccd_enabled(false)
            .position(iso)
            .enabled_rotations(true, true, true)
            .build();

        let builder = ColliderBuilder::cylinder(h * 0.5, r)
            .sensor(true)
            .active_collision_types(ActiveCollisionTypes::all())
            .translation(offset.into())
            .restitution(0.0)
            .restitution_combine_rule(CoefficientCombineRule::Min)
            .friction(2.0)
            .collision_groups(InteractionGroups::new(GROUP_PLAYER.into(), u32::MAX.into()))
            .friction_combine_rule(CoefficientCombineRule::Max);

        let collider = builder.build();

        let body_handle = ps.rigid_body_set.insert(body);

        let collider_handle =
            ps.collider_set
                .insert_with_parent(collider, body_handle, &mut ps.rigid_body_set);

        let physics_handle = {
            let body = ps.rigid_body_set.get_mut(body_handle).unwrap();

            PhysicsHandle {
                rigid_body: body_handle,
                collider: collider_handle,
                og_rb_type: body.body_type(),
            }
        };

        self.physics_handles.insert(parent_id, physics_handle);

        let cyl = Cylinder { r, h };

        let collider_model = cyl.create_model(10);

        self.collider_transforms.insert(
            parent_id,
            Transform {
                position,
                rotation,
                scale: Vec3::ONE,
            },
        );
        self.prev_collider_transforms.insert(
            parent_id,
            Transform {
                position,
                rotation,
                scale: Vec3::ONE,
            },
        );
        self.collider_gizmos.insert(parent_id, collider_model);

        self.collider_to_entity.insert(collider_handle, parent_id);
    }

    pub fn create_sphere_hitbox(
        &mut self,
        r: f32,
        position: Vec3,
        scale: Vec3,
        parent_id: usize,
        ps: &mut PhysicsState,
    ) {
        let sphere = Sphere { r };

        let sphere_mod = sphere.create_model(15, 15, 0.0);

        self.collider_gizmos.insert(parent_id, sphere_mod);
        self.collider_transforms.insert(
            parent_id,
            Transform {
                position,
                rotation: Quat::IDENTITY,
                scale,
            },
        );
        self.prev_collider_transforms.insert(
            parent_id,
            Transform {
                position,
                rotation: Quat::IDENTITY,
                scale,
            },
        );

        let iso: Isometry<f32> = (position, Quat::IDENTITY).into();

        let body = RigidBodyBuilder::dynamic()
            .position(iso)
            .ccd_enabled(true)
            .build();

        let collider = ColliderBuilder::ball(r)
            .sensor(true)
            .active_collision_types(ActiveCollisionTypes::all())
            .active_events(ActiveEvents::COLLISION_EVENTS)
            .mass(1.0)
            .build();

        let body_handle = ps.rigid_body_set.insert(body);

        let collider_handle =
            ps.collider_set
                .insert_with_parent(collider, body_handle, &mut ps.rigid_body_set);

        self.physics_handles.insert(
            parent_id,
            PhysicsHandle {
                rigid_body: body_handle,
                collider: collider_handle,
                og_rb_type: RigidBodyType::Dynamic,
            },
        );

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

        // let mut local_offset = center;

        // local_offset.y = 0.5 * size.y;

        let cuboid = Cuboid {
            w: size.x,
            h: size.y,
            d: size.z,
        };

        let cuboid_model = cuboid.create_model();

        self.collider_gizmos.insert(parent_id, cuboid_model);
        self.model_heights.insert(parent_id, size.y);

        // PHYSICS PASS
        let iso: Isometry<f32> = (position, rotation).into();

        let body = RigidBodyBuilder::kinematic_position_based()
            .position(iso)
            .ccd_enabled(true)
            .build();

        let half_extents = size * 0.5;

        let collider = ColliderBuilder::cuboid(half_extents.x, half_extents.y, half_extents.z)
            .translation(vector![center.x, center.y, center.z])
            .sensor(true)
            .density(0.0)
            .active_events(ActiveEvents::COLLISION_EVENTS)
            .build();

        let body_handle = ps.rigid_body_set.insert(body);
        let collider_handle =
            ps.collider_set
                .insert_with_parent(collider, body_handle, &mut ps.rigid_body_set);
        self.physics_handles.insert(
            parent_id,
            PhysicsHandle {
                rigid_body: body_handle,
                collider: collider_handle,

                og_rb_type: RigidBodyType::KinematicPositionBased,
            },
        );

        self.collider_transforms.insert(
            parent_id,
            Transform {
                position: center,
                rotation: Quat::IDENTITY,
                scale,
            },
        );
        self.prev_collider_transforms.insert(
            parent_id,
            Transform {
                position: center,
                rotation: Quat::IDENTITY,
                scale,
            },
        );

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
        let vertices: Vec<Point3<f32>> = model.vertices.iter().map(|v| v.position.into()).collect();

        let indices: Vec<[u32; 3]> = model
            .indices
            .chunks(3)
            .map(|chunk| [chunk[0], chunk[1], chunk[2]])
            .collect();

        self.collider_transforms.insert(
            parent_id,
            Transform {
                position,
                rotation: Quat::IDENTITY,
                scale,
            },
        );
        self.prev_collider_transforms.insert(
            parent_id,
            Transform {
                position,
                rotation: Quat::IDENTITY,
                scale,
            },
        );

        let iso: Isometry<f32> = (position, rotation).into();

        let body = RigidBodyBuilder::fixed().position(iso).build();

        let collider = ColliderBuilder::trimesh(vertices, indices)
            .unwrap()
            .active_collision_types(ActiveCollisionTypes::all())
            .build();

        let body_handle = ps.rigid_body_set.insert(body);
        let collider_handle =
            ps.collider_set
                .insert_with_parent(collider, body_handle, &mut ps.rigid_body_set);

        self.physics_handles.insert(
            parent_id,
            PhysicsHandle {
                rigid_body: body_handle,
                collider: collider_handle,

                og_rb_type: RigidBodyType::Fixed,
            },
        );

        self.collider_to_entity.insert(collider_handle, parent_id);
    }

    pub fn update(
        &mut self,
        sm: &mut SoundManager,
        ps: &mut PhysicsState,
        input: &mut InputState,
        dt: f32,
        cmds: &mut CommandBuffer,
    ) {
        // ==========================
        // evaluate commands
        // ==========================
        let ecscmds = std::mem::take(&mut cmds.ecs);

        for c in ecscmds {
            match c.action {
                EcsAction::SpawnDamageVolume(anim) => {
                    self.create_damage_volume(c.entity_id, &anim, ps);
                }
            }
        }

        self.tick_weapon_cooldowns(dt);

        for o in self.cleanup_timer.iter_mut() {
            o.value += dt;

            if o.value >= 300.0 {
                // seconds
                self.entity_trashcan.push(o.key());
            }
        }
        if input.just_pressed(KeyCode::Delete) {
            for i in self.selected.iter() {
                self.entity_trashcan.push(*i);
            }
        }
        self.delete_entities(sm, ps);
    }

    /// Tick all weapon ability cooldowns.
    fn tick_weapon_cooldowns(&mut self, dt: f32) {
        for abilities in self.weapon_abilities.iter_mut() {
            abilities.value.tick(dt);
        }
    }

    pub fn delete_entities(&mut self, sm: &mut SoundManager, ps: &mut PhysicsState) {
        for id in &self.entity_trashcan {
            let id = *id;

            // -----------------------------
            // Physics cleanup
            // -----------------------------
            if let Some(ph) = self.physics_handles.get_mut(id) {
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

            // If you keep per-entity hitsets of collider handles, clear them too.
            // (Not strictly required, but keeps memory and references tidy.)
            self.hitsets.remove(id);

            // -----------------------------
            // Core transforms / render gizmos
            // -----------------------------
            self.transforms.remove(id);
            self.prev_transforms.remove(id);
            self.local_corrections.remove(id);
            self.collider_gizmos.remove(id);
            self.collider_transforms.remove(id);
            self.prev_collider_transforms.remove(id);

            // "Area" / gizmo-only dimensions
            self.dimensions.remove(id);

            // -----------------------------
            // Inventory / item ownership relations
            // -----------------------------

            // remove the ownership relation from the inventory items
            if let Some(inv) = self.inventories.get(id) {
                for item_id in inv.iter().copied() {
                    self.owners.remove(item_id);
                    self.is_equipped.remove(item_id);
                    self.cleanup_timer.insert(item_id, 0.0);
                }
            }
            self.inventories.remove(id);

            // drop active held items (weapons etc.)
            if let Some(ai) = self.active_items.get(id) {
                if let Some(rhid) = ai.right_hand {
                    self.owners.remove(rhid);
                    self.is_equipped.remove(rhid);
                    self.cleanup_timer.insert(rhid, 0.0);
                }
                if let Some(lhid) = ai.left_hand {
                    self.owners.remove(lhid);
                    self.is_equipped.remove(lhid);
                    self.cleanup_timer.insert(lhid, 0.0);
                }
            }
            self.active_items.remove(id);

            // sockets/attachment metadata for items
            self.item_bones.remove(id);

            // if the entity itself is owned by something (i.e. it's an item in an inventory),
            // clear that too
            self.owners.remove(id);
            self.is_equipped.remove(id);

            // Abilities assigned to weapon entities
            self.weapon_abilities.remove(id);

            // -----------------------------
            // Gameplay state/components
            // -----------------------------
            self.factions.remove(id);
            self.entity_types.remove(id);
            self.models.remove(id);
            self.animators.remove(id);
            self.skellingtons.remove(id);
            self.rotators.remove(id);
            self.impulse_applied.remove(id);
            self.player_controllers.remove(id);
            self.destinations.remove(id);
            self.v_effects.remove(id);

            self.yaws.remove(id);
            self.knockbacks.remove(id);

            self.healths.remove(id);
            self.max_healths.remove(id);

            self.manas.remove(id);
            self.max_manas.remove(id);

            self.levels.remove(id);
            self.names.remove(id);

            self.base_speeds.remove(id);
            self.aggro_ranges.remove(id);
            self.jump_heights.remove(id);
            self.total_masses.remove(id);
            self.model_heights.remove(id);
            self.grounded_states.remove(id);

            self.pickup_ranges.remove(id);

            self.cleanup_timer.remove(id);

            self.source_ids.remove(id);
            self.lifetimes.remove(id);

            // -----------------------------
            // Misc / bookkeeping
            // -----------------------------

            // Remove from selection list
            self.selected.retain(|&x| x != id);

            // Remove mapping last (after physics removal used it)
            self.physics_handles.remove(id);

            // Sound cleanup last
            sm.cleanup_entity_sounds(id);

            self.current_round_enemies
                .retain(|enemy_id| *enemy_id != id);

            self.damage_volumes.remove(id);
            self.status_effects.remove(id);
        }

        self.entity_trashcan.clear();
    }

    pub fn drop_active_items(&mut self, id: usize) {
        if let Some(ai) = self.active_items.get(id) {
            if let Some(rhid) = ai.right_hand {
                self.owners.remove(rhid);
                self.is_equipped.remove(rhid);
                self.cleanup_timer.insert(rhid, 0.0);
            }
            if let Some(lhid) = ai.left_hand {
                self.owners.remove(lhid);
                self.is_equipped.remove(lhid);
                self.cleanup_timer.insert(lhid, 0.0);
            }
        }
        self.active_items.remove(id);
    }

    pub fn get_ids_for_faction(&self, faction: &str) -> Vec<usize> {
        let result: Vec<usize> = self
            .factions
            .iter()
            .filter_map(|f| {
                if *f.value() == faction {
                    Some(f.key())
                } else {
                    None
                }
            })
            .collect();

        result
    }

    pub fn next_anim_info(&self, id: usize) -> Option<(AnimationType, &Animation)> {
        let Some(animator) = self.animators.get(id) else {
            return None;
        };

        Some((
            animator.next_animation,
            &animator.get_next_animation().unwrap(),
        ))
    }

    /// Returns the entity ID of the player, if one exists.
    pub fn get_player_id(&self) -> Option<usize> {
        self.factions
            .iter()
            .find(|f| *f.value() == "Player")
            .map(|f| f.key())
    }

    /// Player's primary (right-hand) weapon entity, if equipped and configured with [`WeaponAbilities`].
    pub fn player_main_hand_weapon(&self) -> Option<usize> {
        let pid = self.get_player_id()?;
        let wid = self.active_items.get(pid)?.right_hand?;
        self.weapon_abilities.get(wid)?;
        Some(wid)
    }

    pub fn get_ids_for_type(&self, entity_type: &str) -> Vec<usize> {
        let result: Vec<usize> = self
            .entity_types
            .iter()
            .filter_map(|f| {
                if f.value() == entity_type {
                    Some(f.key())
                } else {
                    None
                }
            })
            .collect();

        result
    }

    pub fn get_ids_by_type(&self) -> HashMap<String, Vec<usize>> {
        let mut map: HashMap<String, Vec<usize>> = HashMap::new();

        for entry in self.entity_types.iter() {
            let id = entry.key();
            let ty = &entry.value;

            map.entry(ty.clone()).or_default().push(id);
        }

        map
    }

    pub fn get_all_orphaned_weapon_ids(&self) -> Vec<usize> {
        self.factions
            .iter()
            .filter(|w_type| *w_type.value() == "Item" && self.owners.get(w_type.key()).is_none())
            .map(|e| e.key())
            .collect::<Vec<usize>>()
    }

    /// check if there are any weapons nearby the player
    pub fn has_nearby_weapon(&self) -> bool {
        let player_id = match self.factions.iter().find(|e| *e.value() == "Player") {
            Some(entry) => entry.key(),
            None => return false,
        };

        let pickup_range = self.pickup_ranges.get(player_id).copied().unwrap_or(3.0);

        let player_pos = match self.transforms.get(player_id) {
            Some(t) => t.position,
            None => return false,
        };

        let orphaned_weapons = self.get_all_orphaned_weapon_ids();

        for weapon_id in orphaned_weapons {
            if let Some(weapon_trans) = self.transforms.get(weapon_id) {
                let distance = (weapon_trans.position - player_pos).length();

                if distance <= pickup_range {
                    return true;
                }
            }
        }

        false
    }

    /// try to pick up a nearby weapon for the player
    pub fn try_pickup_weapon(&mut self, ps: &mut crate::physics::PhysicsState) -> bool {
        let player_id = match self.factions.iter().find(|e| *e.value() == "Player") {
            Some(entry) => entry.key(),
            None => return false,
        };

        let pickup_range = self.pickup_ranges.get(player_id).copied().unwrap_or(3.0);

        let player_pos = match self.transforms.get(player_id) {
            Some(t) => t.position,
            None => return false,
        };

        let orphaned_weapons = self.get_all_orphaned_weapon_ids();

        for weapon_id in orphaned_weapons {
            if let Some(weapon_trans) = self.transforms.get(weapon_id) {
                let distance = (weapon_trans.position - player_pos).length();

                if distance <= pickup_range {
                    self.owners.insert(weapon_id, player_id);

                    // remove cleanup timer so it doesn't despawn
                    self.cleanup_timer.remove(weapon_id);

                    // equip the weapon immediately (similar to populate_inventory logic)
                    // unequip current weapon if any
                    if let Some(active) = self.active_items.get(player_id) {
                        if let Some(current_weapon) = active.right_hand {
                            // move current weapon to inventory and mark as unequipped
                            self.is_equipped.remove(current_weapon);
                            match self.inventories.get_mut(player_id) {
                                Some(inv) => {
                                    if !inv.contains(&current_weapon) {
                                        inv.push(current_weapon);
                                    }
                                }
                                None => {
                                    self.inventories.insert(player_id, vec![current_weapon]);
                                }
                            }
                        }
                    }

                    // equip the new weapon
                    self.active_items.insert(
                        player_id,
                        ActiveItem {
                            right_hand: Some(weapon_id),
                            left_hand: None,
                        },
                    );
                    self.is_equipped.insert(weapon_id, true);

                    // change the weapon's physics body from dynamic to kinematic
                    // TODO: handle this better
                    if let Some(ph) = self.physics_handles.get(weapon_id) {
                        if let Some(rb) = ps.rigid_body_set.get_mut(ph.rigid_body) {
                            rb.set_body_type(
                                rapier3d::prelude::RigidBodyType::KinematicPositionBased,
                                false,
                            );
                            rb.set_gravity_scale(0.0, false);
                            rb.enable_ccd(false);
                            rb.wake_up(true);
                        }
                        if let Some(col) = ps.collider_set.get_mut(ph.collider) {
                            col.set_sensor(true);
                            col.set_enabled(true);
                        }
                    }

                    return true;
                }
            }
        }

        false
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

    // pub fn get_non_weapon_entities(&self) -> Vec<usize> {
    //     self.factions
    //         .iter()
    //         .filter(|w_type| {
    //             *w_type.value() != "Item"
    //             //&& *w_type.value() != Faction::World
    //         })
    //         .map(|e| e.key())
    //         .collect::<Vec<usize>>()
    // }

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

    pub fn serialize_entity_data(&self, file_name: &str) {
        let mut wd = WorldData { entities: vec![] };

        for etype in self.entity_types.iter() {
            if etype.value() == "Terrain" {
                continue;
            }

            let id = etype.key();

            match self.owners.get(id) {
                Some(_) => continue,
                None => (),
            }

            let weapons = self.resolve_weapons(id);

            let faction = match self.factions.get(id) {
                Some(f) => Some(f.clone()),
                None => None,
            };

            let jump_height = match self.jump_heights.get(id) {
                Some(jh) => Some(jh.desired),
                _ => None,
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
                max_health: self.max_healths.get(id).copied(),
                mana: self.manas.get(id).copied(),
                max_mana: self.max_manas.get(id).copied(),
                level: self.levels.get(id).copied(),
                name: self.names.get(id).cloned(),
                cleanup_timer: self.cleanup_timer.get(id).copied(),
                pickup_range: self.pickup_ranges.get(id).copied(),
            };

            wd.entities.push(instance);
        }

        wd.save_to_file(file_name);
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
                    faction: Some(self.factions.get(*w).unwrap().clone()),
                    position: Vec3::splat(0.0),
                    rotation: Quat::IDENTITY,
                    weapons: None,
                    base_speed: None,
                    jump_height: None,
                    health: None,
                    max_health: None,
                    mana: None,
                    max_mana: None,
                    level: None,
                    name: None,
                    cleanup_timer: None,
                    pickup_range: None,
                });
            }

            return Some(wlist);
        }
        None
    }

    pub fn remove_entity_type_definition(&mut self, entity_type: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        match std::fs::copy(
            "config/entity_config.json",
            format!("config/archive/entity_config/{}_entity_config.json", now),
        ) {
            Ok(_) => (),
            Err(e) => panic!("Failure: {}", e),
        }
        let mut ids = self.get_ids_for_type(entity_type);

        self.entity_trashcan.append(&mut ids);

        self.entity_type_register.remove(entity_type);

        let ec = EntityConfig {
            entity_types: self.entity_type_register.clone(),
        };

        ec.save_to_file("config/entity_config.json");
    }

    pub fn register_new_entity_type(&mut self, data: &UiEntityTypeHelper) {
        if data.entity_type.is_empty() {
            eprintln!("[Error] Entity Type is blank");
            return;
        }

        if self.entity_type_register.contains_key(&data.entity_type) {
            eprintln!(
                "[Error] Cannot register this entity type, it would overwrite an existing one."
            );
            return;
        }

        let mut etype = EntityTypeHelper::default();

        if !data.mesh_path.is_empty() {
            let texture_path = Path::new(&data.texture_path);
            let mesh_path = Path::new(&data.mesh_path);
            let texture_file_name = texture_path.file_name().unwrap().to_str().unwrap();

            let base_path = Path::new("resources/models/");

            // Get the mesh data just for the memes.
            let file_data = match std::fs::read_to_string(&data.mesh_path) {
                Ok(data) => data,
                Err(_) => {
                    println!("Failed to open mesh file: {}", data.mesh_path);
                    return;
                }
            };

            match file_data.contains("ANIMATION_DATA") {
                true => {
                    let new_mesh_path =
                        base_path.join(&format!("animated/{}/mesh.txt", &data.entity_type));
                    let new_texture_path = base_path.join(&format!(
                        "animated/{}/{}",
                        &data.entity_type, &texture_file_name
                    ));
                    std::fs::create_dir_all(&new_mesh_path.parent().unwrap());
                    std::fs::create_dir_all(&new_texture_path.parent().unwrap());

                    std::fs::copy(mesh_path, &new_mesh_path);
                    std::fs::copy(texture_path, new_texture_path);

                    etype.bone_path = Some(new_mesh_path.clone().to_string_lossy().to_string());
                    etype.mesh_path = new_mesh_path.clone().to_string_lossy().to_string();
                }
                false => {
                    let new_mesh_path =
                        base_path.join(&format!("static/{}/mesh.txt", &data.entity_type));
                    let new_texture_path = base_path.join(&format!(
                        "static/{}/{}",
                        &data.entity_type, &texture_file_name
                    ));

                    std::fs::create_dir_all(&new_mesh_path.parent().unwrap());
                    std::fs::create_dir_all(&new_texture_path.parent().unwrap());

                    std::fs::copy(mesh_path, &new_mesh_path);
                    std::fs::copy(texture_path, new_texture_path);

                    etype.bone_path = None;
                    etype.mesh_path = new_mesh_path.clone().to_string_lossy().to_string();
                }
            }

            if etype.bone_path.is_some() {
                let mut lines = file_data.lines();
                let mut anim_props = vec![];
                while let Some(line) = lines.next() {
                    let parts: Vec<&str> = line.split_whitespace().collect();

                    if parts.is_empty() {
                        continue;
                    }

                    match parts[0] {
                        "ANIMATION_DATA" => {}
                        // TODO: We need to save animation data here;
                        "ANIMATION_NAME:" => match parts[1].trim() {
                            "Idle" => anim_props.push(AnimationPropHelper {
                                name: AnimationType::Idle,
                                one_shots: HashMap::new(),
                                continuous_sounds: vec![],
                                hurtbox_activation: None,
                                hold_frame: None,
                                interrupt_frame: None,
                                reset_on_change: true,
                                do_root_motion: false,
                                projectile_frame: None,
                                damage_volume: None,
                            }),
                            _ => {}
                        },
                        _ => (),
                    }
                }

                if anim_props.len() > 0 {
                    etype.animation_properties = Some(anim_props);
                }
            }
        }

        etype.rot_correction = Quat::from_array(data.rot_correction);
        etype.scale_correction = data.scale_correction.into();

        if data.aggro_range > 0.0 {
            etype.aggro_range = Some(data.aggro_range);
        }

        if data.total_mass > 0.0 {
            etype.total_mass = Some(data.total_mass);
        }

        match data.hitbox.as_str() {
            "Cylinder" => {
                etype.hitbox = HitboxShape::Cylinder {
                    r: data.r,
                    h: data.h,
                };
            }
            "Pill" => {
                etype.hitbox = HitboxShape::Pill {
                    r: data.r,
                    h: data.h,
                };
            }
            "BoundingBox" => {
                etype.hitbox = HitboxShape::BoundingBox;
            }
            "Mesh" => {
                etype.hitbox = HitboxShape::Mesh;
            }
            "Sphere" => {
                etype.hitbox = HitboxShape::Sphere { r: data.r };
            }
            "BoxDim" => {
                etype.hitbox = HitboxShape::BoxDim {
                    hx: data.hx,
                    hy: data.hy,
                    hz: data.hz,
                }
            }
            _ => etype.hitbox = HitboxShape::Mesh,
        }

        self.entity_type_register
            .insert(data.entity_type.clone(), etype);
        let ec = EntityConfig {
            entity_types: self.entity_type_register.clone(),
        };
        ec.save_to_file("config/entity_config.json");
    }

    pub fn register_new_faction(&mut self, faction: &str) {
        self.faction_register.insert(faction.to_string());
    }

    pub fn serialize_faction_register(&self) {
        let cfg = FactionsConfig {
            factions: self.faction_register.clone(),
        };

        cfg.save_to_file("config/factions_config.json");
    }
}

pub fn glam_to_nalgebra_quat(q: Quat) -> UnitQuaternion<f32> {
    UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(q.w, q.x, q.y, q.z))
}

pub fn load_terrain(entity_manager: &mut EntityManager, physics_state: &mut PhysicsState) {
    //let path = "resources/textures/brushes/301B1.png";
    //let path = "resources/textures/brushes/testing.png";
    //let path = "resources/textures/brushes/mountain.png";
    //let path = "resources/textures/brushes/blendertest.png";
    let path = "resources/textures/small_terrain.png";
    let img = assets::load_image(path)
        .expect("Failed to load terrain image")
        .to_luma8();
    let (width, height) = img.dimensions();
    let y_amplitude = 2.5;
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

    entity_manager
        .transforms
        .insert(entity_manager.next_entity_id, terrain_trans.clone());
    //TODO: load this dynamically potentially
    entity_manager
        .factions
        .insert(entity_manager.next_entity_id, "World".to_string());
    entity_manager.faction_register.insert("World".to_string());
    entity_manager
        .entity_types
        .insert(entity_manager.next_entity_id, "Terrain".to_string());

    entity_manager
        .collider_transforms
        .insert(entity_manager.next_entity_id, terrain_trans.clone());

    let iso: Isometry<f32> = (terrain_trans.position, terrain_trans.rotation).into();
    let body = RigidBodyBuilder::fixed().position(iso).build();

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
        width,
        height,
        128,
        128, // how big is each chunk
        body_handle,
        &mut physics_state.collider_set,
        &mut physics_state.rigid_body_set,
    );

    //entity_manager.physics_handles.insert(entity_manager.next_entity_id, PhysicsHandle {
    //    rigid_body: body_handle,
    //    collider: collider_handle,
    //});

    entity_manager
        .models
        .insert(entity_manager.next_entity_id, model);

    entity_manager.next_entity_id += 1;
}
