use glam::{Quat, Vec3};
use nalgebra::{point, vector};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use rapier3d::prelude::{Group, InteractionGroups, QueryFilter, Ray};

use crate::{
    config::world_data::EntityInstance, debug::gizmos::Dimension, entity_manager::EntityManager,
    physics::PhysicsState, util::constants::GROUP_TERRAIN,
};

/// Available weapon entity types for enemy spawns.
const ENEMY_WEAPON_TYPES: &[&str] = &["OrcSword", "DoubleAxe", "IceStaff", "Staff", "FireStaff"];

pub struct SpawnManager {
    pub spawn_every: f32, // seconds
    pub amount_per: u32,  // how many guysf
    pub accumulator: f32,
}

impl SpawnManager {
    pub fn update(&mut self, em: &mut EntityManager, ps: &mut PhysicsState, dt: f32) {
        self.accumulator += dt;

        if self.accumulator >= self.spawn_every {
            self.accumulator -= self.spawn_every;
            match Self::find_spawn_point(em, ps) {
                Some(point) => {
                    for _ in 0..self.amount_per {
                        let weapon_type =
                            ENEMY_WEAPON_TYPES[em.rng.random_range(0..ENEMY_WEAPON_TYPES.len())];

                        let instance = EntityInstance {
                            entity_type: "TrashGuy".to_string(),
                            position: point,
                            faction: Some("Enemy".to_string()),
                            rotation: Quat::IDENTITY,
                            weapons: Some(vec![EntityInstance {
                                entity_type: weapon_type.to_string(),
                                position: Vec3::ZERO,
                                rotation: Quat::IDENTITY,
                                faction: Some("Item".to_string()),
                                base_speed: None,
                                jump_height: None,
                                health: None,
                                max_health: None,
                                mana: None,
                                max_mana: None,
                                level: None,
                                name: None,
                                weapons: None,
                                cleanup_timer: None,
                                pickup_range: None,
                            }]),
                            base_speed: Some(3.5),
                            health: Some(100.0),
                            max_health: None,
                            mana: None,
                            max_mana: None,
                            level: None,
                            name: None,
                            jump_height: Some(1.0),
                            cleanup_timer: None,
                            pickup_range: None,
                        };
                        let parent_id = em.create_mesh_entity(&instance, ps);
                        em.populate_inventory(parent_id, &instance, ps);
                    }
                }
                None => {
                    dbg!("Somehow we didn't get a spawn??? why?????");
                }
            }
        }
    }

    pub fn find_spawn_point(em: &mut EntityManager, ps: &PhysicsState) -> Option<Vec3> {
        let spawn_area_entry = em
            .entity_types
            .iter()
            .find(|e| e.value() == "MainSpawnableArea");

        if spawn_area_entry.is_none() {
            dbg!("Spawn area is missing!");
            return None;
        }

        let spawn_area_id = spawn_area_entry.unwrap().key();
        let trans = em.collider_transforms.get(spawn_area_id).unwrap();
        let dim = em.dimensions.get(spawn_area_id).unwrap();

        let r = match dim {
            Dimension::Cylinder { r, h: _ } => r,
            _ => panic!("we should only pass cyl things into this one"),
        };

        let no_y = Self::random_spot_in_circle(&mut em.rng, trans.position, *r);
        let dir = Vec3::NEG_Y;

        let ray = Ray::new(point![no_y.x, 500.0, no_y.z], vector![dir.x, dir.y, dir.z]);

        let query_pipeline = &ps.query_pipeline.as_ref().unwrap();
        let colliders = &ps.collider_set;
        let bodies = &ps.rigid_body_set;

        let filter =
            QueryFilter::default().groups(InteractionGroups::new(Group::ALL, GROUP_TERRAIN.into()));

        if let Some((_, toi)) = query_pipeline.cast_ray(
            bodies, colliders, &ray, 1000.0, // max ray distance
            true,   // solid?
            filter,
        ) {
            let hit = ray.point_at(toi);
            Some(Vec3::new(hit.x, hit.y + 2.0, hit.z))
        } else {
            None
        }
    }

    pub fn random_spot_in_circle(rng: &mut ChaCha8Rng, origin: Vec3, radius: f32) -> Vec3 {
        // theta is an angle
        let theta = rng.random_range(0.0..std::f32::consts::TAU);

        let u: f32 = rng.random();
        let r = radius * u.sqrt();
        origin + Vec3::new(theta.cos(), 0.0, theta.sin()) * r
    }
}
