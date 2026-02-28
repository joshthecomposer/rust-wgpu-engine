use std::time::Instant;

use crate::{
    animation::{animation::Animation, animator::Animator},
    command_buffer::{CommandBuffer, LocoIntent},
    entity_manager::EntityManager,
    enums_types::{AnimationType, CombatState, ControlState, LocoState},
    input::InputState,
    state_machines::player::{
        combat::combat_state_machine,
        loco::{self, locomotion_state_machine},
    },
    util::constants::{BASIC, DEFENSIVE, EVADE, SKILL1, SKILL2, ULTIMATE},
};

use winit::keyboard::KeyCode;

pub fn player_state_orchestrator(
    em: &mut EntityManager,
    input: &InputState,
    cmds: &mut CommandBuffer,
    dt: f32,
) {
    let Some(player_id) = em.get_player_id() else {
        return;
    };

    let Some(health) = em.healths.get(player_id) else {
        return;
    };

    if *health <= 0.0 {
        // killl!
    }

    let Some(ctrl) = em.player_controllers.get_mut(player_id) else {
        return;
    };

    let Some(animator) = em.animators.get_mut(player_id) else {
        return;
    };

    dbg!(&animator.current_animation);
    dbg!(&animator.next_animation);

    let weap_id = em.active_items.get(player_id).and_then(|w| w.right_hand);

    let Some(gs) = em.grounded_states.get(player_id) else {
        eprintln!("We need a grounded state for player to work right");
        return;
    };

    let trans = em.transforms.get(player_id).unwrap();
    let pos = trans.position;

    match ctrl.control_state {
        ControlState::Player => {
            locomotion_state_machine(ctrl, input, cmds, player_id, weap_id, animator, dt, gs, pos);
        }
        ControlState::Combat => {
            if let Some(weap_id) = weap_id {
                combat_state_machine(ctrl, cmds, player_id, weap_id, input, animator, dt);
            }
        }
        _ => (),
    }
}

pub fn anim_for_combat_state(cs: &Option<CombatState>) -> Option<AnimationType> {
    match cs {
        Some(CombatState::Basic1) => Some(AnimationType::Basic1),
        Some(CombatState::Basic2) => Some(AnimationType::Basic2),
        Some(CombatState::Basic3) => Some(AnimationType::Basic3),
        Some(CombatState::Defensive) => Some(AnimationType::Block),
        Some(CombatState::Skill1) => Some(AnimationType::Basic1),
        Some(CombatState::Skill2) => Some(AnimationType::Basic2),
        Some(CombatState::Evade) => Some(AnimationType::DashF),
        Some(CombatState::Ultimate) => Some(AnimationType::Basic3),
        None => None,
    }
}

pub fn anim_for_loco_state(ls: &LocoState) -> AnimationType {
    match ls {
        LocoState::Init => AnimationType::Idle,
        LocoState::Idle => AnimationType::Idle,
        LocoState::Running => AnimationType::Run,
        LocoState::Jumping => AnimationType::Jump,
        LocoState::Airborne => AnimationType::Freefall,
    }
}

pub fn ability_just_pressed(input: &InputState) -> Option<u32> {
    if input.left_mouse_just_pressed() {
        return Some(BASIC);
    }
    if input.right_mouse_just_pressed() {
        return Some(DEFENSIVE);
    }
    if input.just_pressed(KeyCode::KeyQ) {
        return Some(SKILL1);
    }
    if input.just_pressed(KeyCode::KeyE) {
        return Some(SKILL2);
    }
    if input.just_pressed(KeyCode::ShiftLeft) {
        return Some(EVADE);
    }
    if input.just_pressed(KeyCode::KeyR) {
        return Some(ULTIMATE);
    }

    None
}
