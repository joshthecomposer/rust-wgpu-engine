use crate::{
    animation::animator::{self, Animator},
    command_buffer::{CommandBuffer, LocoIntent},
    entity_manager::EntityManager,
    enums_types::{AnimationType, EnemyController},
    state_machines::enemy::enemy_behavior_tree::ActionKind,
};

pub fn update(em: &mut EntityManager, cmds: &mut CommandBuffer, dt: f32) {
    let enemy_ids = em.get_ids_for_faction("Enemy");

    let player_id = em.get_player_id();

    for eid in enemy_ids {
        let Some(animator) = em.animators.get_mut(eid) else {
            eprintln!("Missing animator");
            continue;
        };

        let Some(anim) = animator.get_next_animation() else {
            eprintln!("Anim not found!");
            return;
        };

        let Some(ctrl) = em.enemy_controllers.get_mut(eid) else {
            continue;
        };

        let weap_id = em.active_items.get(eid).and_then(|w| w.right_hand);

        let next_action = match anim.can_interrupt() {
            false => Some(ctrl.current_action),
            true => ctrl.desired_action,
        };

        match next_action {
            Some(ActionKind::Idle) => {
                cmds.next_anim(eid, AnimationType::Idle, weap_id);
                ctrl.current_action = ActionKind::Idle;
            }
            Some(ActionKind::ChasePlayer) => {
                if let Some(pid) = player_id {
                    let ptrans = em.transforms.get(pid).unwrap();
                    let etrans = em.transforms.get(eid).unwrap();
                    em.destinations.insert(eid, ptrans.position);
                    cmds.next_anim(eid, AnimationType::Run, weap_id);
                    ctrl.current_action = ActionKind::ChasePlayer;
                    let intent = LocoIntent::build_ai_loco_intent(etrans.position, ptrans.position);

                    if !intent.is_zero() {}
                }
            }
            Some(ActionKind::AttackPlayer) => {
                if anim.can_interrupt() {
                    cmds.next_anim_from_lookup(eid, "basic".to_string(), weap_id);
                    ctrl.current_action = ActionKind::AttackPlayer;
                }
            }
            None => {}
        }
    }
}
