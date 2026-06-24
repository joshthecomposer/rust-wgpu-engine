use std::collections::VecDeque;

use glam::{Quat, Vec3};
use nalgebra::{point, vector};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use rapier3d::prelude::{Group, InteractionGroups, QueryFilter, Ray};

use crate::{
    config::{weapon_anim_map, world_data::EntityInstance},
    debug::gizmos::Dimension,
    entity_manager::EntityManager,
    physics::PhysicsState,
    util::constants::GROUP_TERRAIN,
};

// round data for
pub struct RoundData {
    amount: u32,
    spawned: bool,
    weapons: Option<Vec<String>>,
}

pub struct SpawnManager {
    pub rounds: VecDeque<RoundData>,
    pub next_round_ttl: f32,
    pub next_round_accumulator: f32,
}

impl SpawnManager {
    pub fn update(
        &mut self,
        em: &mut EntityManager,
        ps: &mut PhysicsState,
        dt: f32,
        enabled: bool,
    ) {
        if !enabled {
            return;
        }

        let Some(curr) = self.rounds.front_mut() else {
            return;
        };

        if !curr.spawned {
            self.next_round_accumulator += dt;
        }

        if !curr.spawned && self.next_round_accumulator >= self.next_round_ttl {
            Self::spawn_enemies(&curr, em, ps);
            curr.spawned = true;
            self.next_round_accumulator = 0.0;
            return;
        }

        if curr.spawned && em.current_round_enemies.is_empty() {
            self.rounds.pop_front();
            return;
        }
    }

    fn spawn_enemies(round_data: &RoundData, em: &mut EntityManager, ps: &mut PhysicsState) {
        let enemy_weapon_types: Vec<String> =
            em.weapon_anim_map.weapon_types.keys().cloned().collect();

        for i in 0..round_data.amount as usize {
            match Self::find_spawn_point(em, ps) {
                Some(point) => {
                    let weapon_type = if let Some(ref ws) = round_data.weapons {
                        &ws[i]
                    } else {
                        &enemy_weapon_types[em.rng.random_range(0..enemy_weapon_types.len())]
                            .to_string()
                    };

                    let instance = EntityInstance {
                        entity_type: "Peasant1".to_string(),
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
                        health: Some(5.0),
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
                    em.current_round_enemies.push(parent_id);
                }
                None => {
                    dbg!("Somehow we didn't get a spawn??? why????? WHYYYY????");
                }
            }
        }
    }

    fn find_spawn_point(em: &mut EntityManager, ps: &PhysicsState) -> Option<Vec3> {
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

impl Default for SpawnManager {
    fn default() -> Self {
        Self {
            rounds: VecDeque::from([
                RoundData {
                    amount: 1,
                    spawned: false,
                    weapons: Some(vec!["OrcSword".to_string()]),
                    // weapons: None,
                },
                RoundData {
                    amount: 2,
                    spawned: false,
                    weapons: Some(vec!["OrcSword".to_string(), "DoubleAxe".to_string()]),
                },
                RoundData {
                    amount: 4,
                    spawned: false,
                    weapons: Some(vec![
                        "OrcSword".to_string(),
                        "OrcSword".to_string(),
                        "DoubleAxe".to_string(),
                        "FireStaff".to_string(),
                    ]),
                },
            ]),
            next_round_ttl: 5.0,
            next_round_accumulator: 0.0,
        }
    }
}
