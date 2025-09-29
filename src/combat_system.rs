use glam::{vec3, Vec3};

use crate::{entity_manager::EntityManager, enums_types::{AnimationType, AttackState, Effect, Faction, Knockback, PlayerState, SimState, VisualEffect}, physics::PhysicsState};

pub fn update(
    em: &mut EntityManager,
    dt: f32,
    ps: &mut PhysicsState,
) {
    handle_player_to_enemy(em, ps);
    handle_enemy_to_player(em, ps);
}

fn handle_player_to_enemy(em: &mut EntityManager, ps: &mut PhysicsState) {
    let attacking_player_ids = em.player_get_ids_for_state(PlayerState::Attacking);

    for player_id in attacking_player_ids {
        let yaw = em.yaws.get(player_id).unwrap();
        let active_item = em.active_items.get(player_id).unwrap();
        let rh_id = active_item.right_hand.unwrap(); // Only doing right hand weeapons right now.
        let hitset = em.hitsets.get_mut(rh_id).unwrap();

        let rh_parent = em.parents.iter().find(|p| p.value().parent_id == rh_id).unwrap();
        let rh_w_col_handle = em.collider_to_entity.iter().find(|c| *c.1 == rh_parent.key()).unwrap().0;

        let controller = em.player_controllers.get(player_id).unwrap();

        let strength = match controller.attack_state {
            AttackState::Attack2 => 4.3,
            _=> 2.5
        };


        let player_cyl_handle = em.physics_handles.get(player_id).unwrap().collider;
        
        let slash = em.animators
            .get(player_id)
            .unwrap()
            .animations
            .get(&AnimationType::Slash)
            .unwrap();

        let slash2 = em.animators
            .get(player_id)
            .unwrap()
            .animations
            .get(&AnimationType::Slash2)
            .unwrap();

        let active =
        slash.hurtbox_activation.as_ref().map_or(false, |ha| ha.triggered.get()) ||
        slash2.hurtbox_activation.as_ref().map_or(false, |ha| ha.triggered.get());

        if !active {
            hitset.clear();               // important: reset when inactive
            return;                       // skip this frame
        }

        for (c1, c2, i) in ps.narrow_phase.intersection_pairs_with(*rh_w_col_handle) {
            if i {
                if c1 == player_cyl_handle || c2 == player_cyl_handle { continue; }

                let other = if c1 == *rh_w_col_handle { c2 } else { c1 };
                let target_id = *em.collider_to_entity.get(&other).unwrap();

                match em.factions.get(target_id) {
                    Some(faction) => {
                        if *faction != Faction::Enemy {
                            continue;
                        }
                    },
                    None => { continue },
                };

                // if *faction != Faction::Enemy { continue; }

                let sim_state = em.simstate_controllers.get(target_id).unwrap();

                if !hitset.insert(other) { continue };

                if let Some(ph) = em.physics_handles.get(target_id) {
                    if let Some(rb) = ps.rigid_body_set.get_mut(ph.rigid_body) {

                        let mut kb = Knockback {
                            ttl: 0.35,
                            flinch: false,
                        };

                        if sim_state.state != SimState::Blocking {
                            if let Some(h) = em.healths.get_mut(target_id) { *h -= 50.0 };

                            em.v_effects.insert(target_id, VisualEffect {
                                effect: Effect::Flashing,
                                ttl: kb.ttl,
                            });

                            kb.flinch = true;
                        }
                        let dir = vec3(yaw.sin(), 1.0, yaw.cos()).normalize();
                        rb.apply_impulse((dir * strength).into(), true);
                        em.knockbacks.insert(target_id, kb);
                    }
                }
            }
        }
    }
}

fn handle_enemy_to_player(em: &mut EntityManager, ps: &mut PhysicsState) {
    let attacking_enemy_ids = em.enemy_get_ids_for_state(SimState::Attacking);

    for entity_id in attacking_enemy_ids {
        let yaw = em.yaws.get(entity_id).unwrap();

        let active_item = em.active_items.get(entity_id).unwrap();
        let rh_id = active_item.right_hand.unwrap(); // Only doing right hand weeapons right now.
        let hitset = em.hitsets.get_mut(rh_id).unwrap();

        let rh_parent = em.parents.iter().find(|p| p.value().parent_id == rh_id).unwrap();
        let rh_w_col_handle = em.collider_to_entity.iter().find(|c| *c.1 == rh_parent.key()).unwrap().0;

        let entity_cyl_handle = em.physics_handles.get(entity_id).unwrap().collider;

        let controller = em.simstate_controllers.get(entity_id).unwrap();

        let strength = match controller.attack_state {
            AttackState::Attack2 => 4.3,
            _=> 2.5
        };

        let anim = em.animators
            .get(entity_id)
            .unwrap()
            .animations
            .get(&AnimationType::Slash)
            .unwrap();

        let active = anim
            .hurtbox_activation
            .as_ref()
            .map_or(false, |ha| ha.triggered.get());

        if !active {
            hitset.clear();               // important: reset when inactive
            return;                       // skip this frame
        }

        for (c1, c2, i) in ps.narrow_phase.intersection_pairs_with(*rh_w_col_handle) {
            if i {
                if c1 == entity_cyl_handle || c2 == entity_cyl_handle { continue; }

                let other = if c1 == *rh_w_col_handle { c2 } else { c1 };
                let target_id = *em.collider_to_entity.get(&other).unwrap();

                let faction = em.factions.get(target_id).unwrap();
                if *faction != Faction::Player { continue; }

                let player_state = em.player_controllers.get(target_id).unwrap();

                if !hitset.insert(other) { continue };

                if let Some(ph) = em.physics_handles.get(target_id) {
                    if let Some(rb) = ps.rigid_body_set.get_mut(ph.rigid_body) {
                        let mut kb = Knockback {
                            ttl: 0.35,
                            flinch: false,
                        };
                        if player_state.state != PlayerState::Blocking {
                            if let Some(h) = em.healths.get_mut(target_id) { *h -= 50.0 };

                            em.v_effects.insert(target_id, VisualEffect {
                                effect: Effect::Flashing,
                                ttl: kb.ttl,
                            });

                            kb.flinch = true;
                        }
                        let dir = vec3(yaw.sin(), 1.0, yaw.cos()).normalize();
                        rb.apply_impulse((dir * strength).into(), true);
                        em.knockbacks.insert(target_id, kb);
                    }
                }
            }
        }

    }
}
