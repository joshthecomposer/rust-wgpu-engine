use glam::vec3;

use crate::{
    entity_manager::EntityManager,
    enums_types::{AnimationType, AttackState, Knockback, PlayerState, SimState},
    particles::ParticleSystem,
    physics::{self, PhysicsState},
};

pub fn update(em: &mut EntityManager, _dt: f32, ps: &mut PhysicsState, px: &mut ParticleSystem) {
    handle_player_to_enemy(em, ps, px);
    handle_enemy_to_player(em, ps);
}

fn handle_player_to_enemy(
    em: &mut EntityManager,
    ps: &mut PhysicsState,
    _particles: &mut ParticleSystem,
) {
    let attacking_player_ids = em.player_get_ids_for_state(PlayerState::Combat);

    for player_id in attacking_player_ids {
        let yaw = em.yaws.get(player_id).unwrap();
        let active_weapon_id = em.active_items.get(player_id).unwrap().right_hand.unwrap();

        let hitset = em.hitsets.get_mut(active_weapon_id).unwrap();

        let rh_w_col_handle = em.physics_handles.get(active_weapon_id).unwrap().collider;

        let controller = em.player_controllers.get(player_id).unwrap();

        let kb_distance = match controller.attack_state {
            AttackState::Attack2 => 3.5,
            _ => 2.0,
        };

        let player_cyl_handle = em.physics_handles.get(player_id).unwrap().collider;

        let slash = em
            .animators
            .get(player_id)
            .unwrap()
            .animations
            .get(&AnimationType::Slash)
            .unwrap();

        let slash2 = em
            .animators
            .get(player_id)
            .unwrap()
            .animations
            .get(&AnimationType::Slash2)
            .unwrap();

        let active = slash
            .hurtbox_activation
            .as_ref()
            .map_or(false, |ha| ha.triggered.get())
            || slash2
                .hurtbox_activation
                .as_ref()
                .map_or(false, |ha| ha.triggered.get());

        if !active {
            hitset.clear(); // important: reset when inactive
            return; // skip this frame
        }

        for (c1, c2, i) in ps.narrow_phase.intersection_pairs_with(rh_w_col_handle) {
            if i {
                if c1 == player_cyl_handle || c2 == player_cyl_handle {
                    continue;
                }

                let other = if c1 == rh_w_col_handle { c2 } else { c1 };

                // Target id is the pill entity.
                let Some(&target_id) = em.collider_to_entity.get(&other) else {
                    eprintln!(
                        "[combat] collider {:?} has no entity; likely stale pair or missing insert",
                        other
                    );
                    continue;
                };

                match em.factions.get(target_id) {
                    Some(faction) => {
                        if *faction != "Enemy" {
                            continue;
                        }
                    }
                    None => continue,
                };

                //if *faction != Faction::Enemy { continue; }

                let sim_state = em.simstate_controllers.get(target_id).unwrap();

                if !hitset.insert(other) {
                    continue;
                };

                if let Some(ph) = em.physics_handles.get(target_id) {
                    if let Some(rb) = ps.rigid_body_set.get_mut(ph.rigid_body) {
                        let mut kb = Knockback {
                            ttl: 0.35,
                            flinch: false,
                            did_particles: false,
                        };

                        // let trans = em.transforms.get(target_id).unwrap();

                        if sim_state.state != SimState::Blocking {
                            if let Some(h) = em.healths.get_mut(target_id) {
                                *h -= 50.0
                            };

                            // em.v_effects.insert(target_id, VisualEffect {
                            //     effect: Effect::Flashing,
                            //     ttl: kb.ttl,
                            // });

                            kb.flinch = true;
                        }

                        let dir = vec3(yaw.sin(), 1.0, yaw.cos()).normalize();
                        physics::apply_delta_v(rb, dir, kb_distance);
                        em.knockbacks.insert(target_id, kb);
                    }
                }
            }
        }
    }
}

fn handle_enemy_to_player(em: &mut EntityManager, ps: &mut PhysicsState) {
    let attacking_enemy_ids = em.enemy_get_ids_for_state(SimState::Combat);

    for entity_id in attacking_enemy_ids {
        let yaw = em.yaws.get(entity_id).unwrap();
        let active_weapon_id = em.active_items.get(entity_id).unwrap().right_hand.unwrap();

        let hitset = em.hitsets.get_mut(active_weapon_id).unwrap();

        let rh_w_col_handle = em.physics_handles.get(active_weapon_id).unwrap().collider;

        let controller = em.simstate_controllers.get(entity_id).unwrap();

        let kb_distance = match controller.attack_state {
            AttackState::Attack2 => 3.5,
            _ => 2.0,
        };

        let entity_cyl_handle = em.physics_handles.get(entity_id).unwrap().collider;

        let slash = em
            .animators
            .get(entity_id)
            .unwrap()
            .animations
            .get(&AnimationType::Slash)
            .unwrap();

        let slash2 = em
            .animators
            .get(entity_id)
            .unwrap()
            .animations
            .get(&AnimationType::Slash2)
            .unwrap();

        let active = slash
            .hurtbox_activation
            .as_ref()
            .map_or(false, |ha| ha.triggered.get())
            || slash2
                .hurtbox_activation
                .as_ref()
                .map_or(false, |ha| ha.triggered.get());

        if !active {
            hitset.clear(); // important: reset when inactive
            return; // skip this frame
        }

        for (c1, c2, i) in ps.narrow_phase.intersection_pairs_with(rh_w_col_handle) {
            if i {
                if c1 == entity_cyl_handle || c2 == entity_cyl_handle {
                    continue;
                }

                let other = if c1 == rh_w_col_handle { c2 } else { c1 };

                let Some(&target_id) = em.collider_to_entity.get(&other) else {
                    eprintln!(
                        "[combat] collider {:?} has no entity; likely stale pair or missing insert",
                        other
                    );
                    continue;
                };

                match em.factions.get(target_id) {
                    Some(faction) => {
                        if *faction != "Player" {
                            continue;
                        }
                    }
                    None => continue,
                };

                let player_state = em.player_controllers.get(target_id).unwrap();

                if !hitset.insert(other) {
                    continue;
                };

                if let Some(ph) = em.physics_handles.get(target_id) {
                    if let Some(rb) = ps.rigid_body_set.get_mut(ph.rigid_body) {
                        let mut kb = Knockback {
                            ttl: 0.35,
                            flinch: false,
                            did_particles: false,
                        };
                        if player_state.state != PlayerState::Block {
                            if let Some(h) = em.healths.get_mut(target_id) {
                                *h -= 50.0
                            };

                            // em.v_effects.insert(target_id, VisualEffect {
                            //     effect: Effect::Flashing,
                            //     ttl: kb.ttl,
                            // });

                            kb.flinch = true;
                        }
                        let dir = vec3(yaw.sin(), 1.0, yaw.cos()).normalize();
                        physics::apply_delta_v(rb, dir, kb_distance);
                        em.knockbacks.insert(target_id, kb);
                    }
                }
            }
        }
    }
}
