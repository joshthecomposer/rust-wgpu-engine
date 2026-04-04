use crate::{
    command_buffer::CommandBuffer, entity_manager::EntityManager, enums_types::AnimationType,
    state_machines::enemy::enemy_behavior_tree::ActionKind,
};

pub fn update(em: &mut EntityManager, cmds: &mut CommandBuffer) {
    for entry in em.enemy_controllers.iter() {
        let eid = entry.key();
        let ctrl = entry.value();

        let Some(pid) = em.get_player_id() else {
            continue;
        };

        let ptrans = em.transforms.get(pid).unwrap();

        let Some(action) = &ctrl.desired_action else {
            continue;
        };

        let maybe_weap_id = em.active_items.get(eid).and_then(|a| a.right_hand);

        let animator = em.animators.get_mut(eid).unwrap();
        let anim = animator.get_next_animation().unwrap();

        if !anim.can_interrupt() {
            continue;
        }

        dbg!(&action);

        match action {
            ActionKind::Idle => cmds.next_anim(eid, AnimationType::Idle, maybe_weap_id),
            ActionKind::ChasePlayer => {
                cmds.next_anim(eid, AnimationType::Run, maybe_weap_id);
                em.destinations.insert(eid, ptrans.position);
            }
            ActionKind::AttackPlayer => {
                cmds.next_anim_from_lookup(eid, "basic".to_string(), maybe_weap_id);
                em.destinations.remove(eid);
            }
        }
    }
}
