use std::time::Instant;

use glam::Vec3;

use crate::{
    entity_manager::EntityManager,
    enums_types::{CombatState, ControlState, LifeState},
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

        if ctrl.took_damage {
            continue;
        }

        if ctrl.life_state != LifeState::Alive {
            continue;
        }

        if let Some(pid) = player_id {
            let player_pos = em.transforms.get(pid).unwrap().position;
            let p_animator = em.animators.get(pid).unwrap();
            let p_anim = p_animator.get_next_animation().unwrap();

            let entity_trans = em.transforms.get(id).unwrap();

            ctx.can_see_player = true;

            ctx.is_in_melee_range = entity_trans.position.distance(player_pos) <= 1.15;
            ctx.is_in_projectile_range = entity_trans.position.distance(player_pos) <= 25.0;

            ctx.is_in_aggro_range = true;

            ctx.player_is_attacking = p_anim.hurtbox_activation.as_ref().is_some_and(|hba| {
                let current_segment = p_anim.current_segment.get();

                hba.iter()
                    .any(|fa| fa.segment_range.contains(&current_segment))
            });
        }

        bt.update(&mut ctx);

        ctrl.desired_action = ctx.desired_action;
    }
}
