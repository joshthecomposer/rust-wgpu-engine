use std::time::Instant;

use glam::Vec3;

use crate::{
    entity_manager::EntityManager,
    enums_types::{CombatState, ControlState},
    state_machines::enemy::enemy_behavior_tree::BtContext,
};

pub fn update(em: &mut EntityManager) {
    let enemy_ids = em.get_ids_for_faction("Enemy");

    let player_id = em.get_player_id();

    for id in enemy_ids {
        let Some(bt) = em.behavior_trees.get_mut(id) else {
            eprintln!("Could not find behavior tree for enemy entity");
            return;
        };

        let mut ctx = BtContext::default();
        let ctrl = em.enemy_controllers.get_mut(id).unwrap();

        if let Some(pid) = player_id {
            let player_pos = em.transforms.get(pid).unwrap().position;
            let p_animator = em.animators.get(pid).unwrap();
            let p_anim = p_animator.get_next_animation().unwrap();

            let entity_trans = em.transforms.get(id).unwrap();
            let pctrl = em.player_controllers.get(pid).unwrap();

            let fov_threshold = 0.5;
            let to_player = (player_pos - entity_trans.position)
                .with_y(0.0)
                .normalize_or_zero();

            ctx.can_see_player = {
                let forward = (entity_trans.rotation * Vec3::Z)
                    .with_y(0.0)
                    .normalize_or_zero();
                let alignment = forward.dot(to_player);

                alignment >= fov_threshold
            };

            ctx.was_recently_damaged = ctrl.took_damage;
            ctx.is_in_melee_range = entity_trans.position.distance(player_pos) <= 1.0;
            ctx.is_in_aggro_range = if let Some(ar) = em.aggro_ranges.get(id) {
                entity_trans.position.distance(player_pos) <= *ar
            } else {
                false
            };

            ctx.player_is_attacking = if let Some(hba) = &p_anim.hurtbox_activation {
                if p_anim.current_segment.get() <= *hba.segment_range.end() {
                    true
                } else {
                    false
                }
            } else {
                false
            }
        }

        bt.update(&mut ctx);

        ctrl.desired_action = ctx.desired_action;
    }
}
