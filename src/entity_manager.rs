#![allow(clippy::too_many_arguments)]

use core::f32;
use std::collections::HashSet;

use glam::{Mat4, Quat, Vec3};
use nalgebra::{Point3, UnitQuaternion};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rapier3d::{control::KinematicCharacterController, parry::{shape::Capsule, utils::hashmap::HashMap}, prelude::*};

use crate::{animation::{self, animation::{Animation, Animator, Bone, Model}}, config::{entity_config::{AnimationPropHelper, EntityConfig, EntityTypeHelper, ItemBones}, world_data::{EntityInstance, WorldData}}, debug::gizmos::{Cuboid, Cylinder, Pill}, enums_types::{ActiveItem, AttackState, EntityType, EquipSlot, Faction, FrameActivation, HitboxShape, Inventory, Knockback, Parent, PhysicsHandle, PlayerController, PlayerState, Rotator, SimState, SimStateController, Transform, VisualEffect}, physics::PhysicsState, sound::sound_manager::{ContinuousSound, OneShot, SoundManager}, sparse_set::SparseSet, terrain::Terrain};

pub struct EntityManager {
    pub next_entity_id: usize,
    pub transforms: SparseSet<Transform>,
    pub prev_transforms: SparseSet<Transform>,
    pub parents: SparseSet<usize>,
    pub children: SparseSet<Vec<usize>>,

    pub collider_to_parent: HashMap<ColliderHandle, usize>,

    // The presense of this determines that it is equipped and where
    // query the parent for further info
    pub equip_slots: SparseSet<EquipSlot>,
    // separate from the parent, this implies the inventory of the parent
    pub owners: SparseSet<usize>,
    // sockets for items to attach to, parent to a bone instead of the parent of the bone
    pub item_bones: SparseSet<ItemBones>,

    pub factions: SparseSet<Faction>,
    pub entity_types: SparseSet<EntityType>,
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

}

impl EntityManager {
    pub fn new(max_entities: usize) -> Self {
        Self {
            next_entity_id: 0,
            transforms: SparseSet::with_capacity(max_entities),
            prev_transforms: SparseSet::with_capacity(max_entities),

            collider_to_parent: HashMap::new(),


            parents: SparseSet::with_capacity(max_entities),
            children: SparseSet::with_capacity(max_entities),
            equip_slots: SparseSet::with_capacity(max_entities),
            owners: SparseSet::with_capacity(max_entities),

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

        }
    }

    pub fn populate_entity_data(&mut self, ps: &mut PhysicsState) {
        let ec = EntityConfig::load_from_file("config/entity_config.json");
        let wd = WorldData::load_from_file("config/world_data.toml");

        for instance in wd.entities.iter() {
            let archetype = ec.entity_types.get(&instance.entity_type).unwrap();

            let parent_id = self.create_entity(archetype, instance, ps);
            self.populate_inventory(parent_id, &instance, &ec, ps);
        }

        load_terrain(self, ps);
    }

    pub fn populate_inventory(&mut self, parent_id: usize, instance: &EntityInstance, ec: &EntityConfig, ps: &mut PhysicsState) {
        if let Some(weapons_list) = &instance.weapons {
            for weapon in weapons_list.iter() {

                let wi = EntityInstance {
                    entity_type: weapon.clone(),
                    faction: Faction::Item,
                    position: Vec3::splat(0.0),
                    rotation: Quat::IDENTITY,
                    weapons: None,
                    base_speed: None,
                    health: None,
                };

                let weapon_archetype = ec.entity_types.get(&wi.entity_type).unwrap();

                let weapon_id = self.create_entity(
                    weapon_archetype,
                    &wi, 
                    ps,
                );

                self.hitsets.insert(
                    weapon_id,
                    HashSet::new(),
                );

                self.parents.insert(weapon_id, parent_id);
                self.owners.insert(weapon_id, parent_id);

                let already_has = self.owners
                    .iter()
                    .filter(|o| *o.value() == parent_id)
                    .any(|e| self.equip_slots.get(e.key()).is_some());

                if !already_has {
                    self.equip_slots.insert(weapon_id, EquipSlot::RHand);
                }

                if let Some(maybe_children) = self.children.get_mut(parent_id) {
                    maybe_children.push(weapon_id);
                } else {
                    self.children.insert(parent_id, vec![weapon_id]);
                }
            }
        }
    }

    pub fn create_entity(
        &mut self, 
        archetype: &EntityTypeHelper, 
        instance: &EntityInstance, 
        ps: &mut PhysicsState,
    ) -> usize {
        let parent_id = self.next_entity_id;
        let position = instance.position;
        let rotation = instance.rotation;
        let scale    = archetype.scale_correction;

        match instance.faction {
            Faction::Player => {
                self.player_controllers.insert(parent_id, PlayerController {
                    state: PlayerState::Init,
                    attack_state: AttackState::Attack1,
                    time_in_state: 0.0,
                });
            },
            _=> ()
        }

        self.factions.insert(self.next_entity_id, instance.faction.clone());
        self.entity_types.insert(self.next_entity_id, instance.entity_type.clone());

        if let Some(health) = instance.health {
            self.healths.insert(parent_id, health);
        }

        let transform = Transform {
            position,
            rotation,
            scale,
        };
        self.transforms.insert(self.next_entity_id, transform.clone());

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
        let cyl_pos = position;
        // === PHYSICS ===
        let iso: Isometry<f32> = (cyl_pos, rotation).into();
        let body = RigidBodyBuilder::kinematic_position_based()
            .ccd_enabled(true)
            .position(iso)
            .enabled_rotations(true, true, true)
            .build();

        let capsule_total_height = h;
        let capsule_half_height = (capsule_total_height - 2.0 * r) / 2.0;

        let offset = 0.039;

        let collider = ColliderBuilder::capsule_y(capsule_half_height, r)
            // let collider = ColliderBuilder::cylinder(cyl.h * 0.5, cyl.r)
            .active_collision_types(ActiveCollisionTypes::all())
            // TODO: This is a hacky way to fix the fact that colliders are centered at half height
            // by default. Likely there is a better way to fix this?
            .translation(vector![0.0, capsule_half_height + r, 0.0]) 
            .build();

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

        let collider_id = self.next_entity_id;

        let collider_model = Pill {
            r,
            h: capsule_total_height,
        }.create_model(12, 5, offset);

        self.transforms.insert(collider_id, Transform {
            position: cyl_pos,
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(1.0),
        });
        self.models.insert(collider_id, collider_model);
        self.entity_types.insert(collider_id, EntityType::Cylinder);
        self.factions.insert(collider_id, Faction::Gizmo);
        self.parents.insert(collider_id, parent_id);
        
        if let Some(maybe_children) = self.children.get_mut(parent_id) {
            maybe_children.push(collider_id);
        } else {
            self.children.insert(parent_id, vec![collider_id]);
        }

        self.collider_to_parent.insert(collider_handle, collider_id);

        self.next_entity_id += 1;
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

        self.models.insert(self.next_entity_id, cyl_mod);
        self.factions.insert(self.next_entity_id, Faction::Gizmo);
        self.entity_types.insert(self.next_entity_id, EntityType::Cylinder);
        self.transforms.insert(self.next_entity_id, Transform {
            position,
            rotation: Quat::IDENTITY,
            scale,
        });

        self.parents.insert(self.next_entity_id, parent_id);
        if let Some(maybe_children) = self.children.get_mut(parent_id) {
            maybe_children.push(self.next_entity_id);
        } else {
            self.children.insert(parent_id, vec![self.next_entity_id]);
        }

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

        self.physics_handles.insert(self.next_entity_id, PhysicsHandle {
            rigid_body: body_handle,
            collider: collider_handle,
        });

        self.collider_to_parent.insert(collider_handle, self.next_entity_id);

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

        self.models.insert(self.next_entity_id, cuboid_model);
        self.factions.insert(self.next_entity_id, Faction::Gizmo);
        self.entity_types.insert(self.next_entity_id, EntityType::Cuboid);
        self.parents.insert(self.next_entity_id, parent_id);


        self.transforms.insert(self.next_entity_id, Transform {
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
        });

        self.collider_to_parent.insert(collider_handle, self.next_entity_id);

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
        self.delete_entities(sm);
    }

   pub fn delete_entities(&mut self, sm: &mut SoundManager) {
       // TODO: Also clean up colliders from here.
       for id in self.entity_trashcan.iter() {
           // sm.cleanup_entity_sounds(*id);
           self.transforms.remove(*id);
           self.factions.remove(*id);
           self.entity_types.remove(*id);
           self.models.remove(*id);
           self.animators.remove(*id);
           self.skellingtons.remove(*id);
           self.rotators.remove(*id);
           self.simstate_controllers.remove(*id);
           self.destinations.remove(*id);
           self.parents.remove(*id);
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

    pub fn get_all_orphaned_weapon_ids(&self) -> Vec<usize> {
        self.factions
            .iter()
            .filter(|w_type| {
                *w_type.value() == Faction::Item 
                    && self.owners.get(w_type.key()).is_none()
                    && self.parents.get(w_type.key()).is_none()
            })
            .map(|e| e.key())
            .collect::<Vec<usize>>()
    }

    pub fn get_active_weapon_ids(&self) -> Vec<usize> {
        self.factions
            .iter()
            .filter(|entry| {
                *entry.value() == Faction::Item
                    && self.equip_slots.get(entry.key()).is_some()
            })
            .map(|e| e.key())
            .collect()
    }

    pub fn get_equipped_weapon_ids(&self) -> Vec<usize> {
        self.equip_slots.iter().map(|e| e.key()).collect()
    }
}

pub fn glam_to_nalgebra_quat(q: Quat) -> UnitQuaternion<f32> {
    UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(q.w, q.x, q.y, q.z))
}

pub fn load_terrain(entity_manager: &mut EntityManager, physics_state: &mut PhysicsState) {
        let mut terrain = Terrain::from_height_map("resources/textures/brushes/301B1.png");

        let model = terrain.into_opengl_model();

        let terrain_trans = Transform {
            position: Vec3::splat(0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(1.0),
        };

        entity_manager.transforms.insert(entity_manager.next_entity_id, terrain_trans.clone(), );
        entity_manager.factions.insert(entity_manager.next_entity_id, Faction::World);
        entity_manager.entity_types.insert(entity_manager.next_entity_id, EntityType::Terrain);

        // Terrain collider
        let terrain_trans = Transform {
            position: Vec3::new(0.0, -0.5, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(1.0),
        };

        entity_manager.transforms.insert(entity_manager.next_entity_id, terrain_trans.clone());
        entity_manager.factions.insert(entity_manager.next_entity_id, Faction::World);
        entity_manager.entity_types.insert(entity_manager.next_entity_id, EntityType::Terrain);


        // Make a big static cube collider
        let iso: Isometry<f32> = (terrain_trans.position, terrain_trans.rotation).into();
        let body = RigidBodyBuilder::fixed().position(iso).build();

        // Process vertices into arrays
        let vertices: Vec<Point3<f32>> = model.vertices
            .iter()
            .map(|v| v.position.into())
            .collect();
        
        let indices: Vec<[u32; 3]> = model.indices
            .chunks(3)
            .map(|chunk| [chunk[0], chunk[1], chunk[2]])
            .collect();

        let terrain_collider = ColliderBuilder::trimesh(vertices, indices).unwrap();
        // let terrain_collider = ColliderBuilder::cuboid(50.0, 0.5, 50.0).build();

        let body_handle = physics_state.rigid_body_set.insert(body);
        let collider_handle = physics_state.collider_set.insert_with_parent(
            terrain_collider,
            body_handle,
            &mut physics_state.rigid_body_set,
        );

        entity_manager.physics_handles.insert(entity_manager.next_entity_id, PhysicsHandle {
            rigid_body: body_handle,
            collider: collider_handle,
        });

        entity_manager.models.insert(entity_manager.next_entity_id, model);

        entity_manager.next_entity_id += 1;
}
