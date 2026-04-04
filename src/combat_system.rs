use glam::vec3;

use crate::{
    entity_manager::EntityManager,
    enums_types::Knockback,
    particles::ParticleSystem,
    physics::{self, PhysicsState},
};

pub fn update(em: &mut EntityManager, _dt: f32, ps: &mut PhysicsState) {
    handle_melee_hits(em, ps);
}

fn handle_melee_hits(em: &mut EntityManager, ps: &mut PhysicsState) {
    let Some(player_id) = em.get_player_id() else {
        eprintln!("There is no player");
        return;
    };

    let player_pill_handle = match em.physics_handles.get(player_id) {
        Some(handle) => handle.collider,
        None => return eprintln!("Player has no pill handle??"),
    };

    let yaw = em.yaws.get(player_id).unwrap();
    let active_weapon_id = em.active_items.get(player_id).unwrap().right_hand.unwrap();

    let hitset = em.hitsets.get_mut(active_weapon_id).unwrap();

    let animator = em.animators.get(player_id).unwrap();

    let active = animator
        .get_next_animation()
        .and_then(|anim| anim.hurtbox_activation.as_ref())
        .map_or(false, |ha| ha.triggered.get());

    if !active {
        hitset.clear();
        return;
    }

    let rh_w_col_handle = em.physics_handles.get(active_weapon_id).unwrap().collider;

    for (c1, c2, i) in ps.narrow_phase.intersection_pairs_with(rh_w_col_handle) {
        if i {
            if c1 == player_pill_handle || c2 == player_pill_handle {
                continue;
            }

            let other = if c1 == rh_w_col_handle { c2 } else { c1 };

            let Some(&target_id) = em.collider_to_entity.get(&other) else {
                eprintln!(
                    "collider {:?} has no entity; likely stale pair or missing insert",
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

            if !hitset.insert(other) {
                continue;
            };

            if let Some(ph) = em.physics_handles.get(target_id) {
                if let Some(rb) = ps.rigid_body_set.get_mut(ph.rigid_body) {
                    let kb = Knockback {
                        ttl: 0.35,
                        flinch: false,
                        did_particles: false,
                    };

                    let enemy_ctrl = em.enemy_controllers.get_mut(target_id).unwrap();

                    enemy_ctrl.took_damage = true;

                    let dir = vec3(yaw.sin(), 1.0, yaw.cos()).normalize();

                    physics::apply_delta_v(rb, dir, 1.5);
                    em.knockbacks.insert(target_id, kb);
                }
            }
        }
    }
}
