use std::time::Instant;

use glam::{vec3, Vec3};
use winit::keyboard::KeyCode;

use crate::{
    animation::animator::Animator,
    command_buffer::{CommandBuffer, ImpulseKind, LocoIntent, PartCmd, PartKind},
    enums_types::{
        AnimationType, CombatState, ControlState, LocoState, PlayerController, ANIMATION_EPSILON,
    },
    input::InputState,
    state_machines::player::{
        loco::ability_to_anim_lookup,
        orchestrator::{ability_just_pressed, anim_for_combat_state},
    },
    util::constants::{BASIC, DEFENSIVE, EVADE, SKILL1, SKILL2, ULTIMATE},
};

pub fn combat_state_machine(
    ctrl: &mut PlayerController,
    cmds: &mut CommandBuffer,
    player_id: usize,
    weap_id: usize,
    input: &InputState,
    dt: f32,
) {
    ctrl.combat_time += dt;

    let Some(action) = (if input.right_mouse_is_down() {
        Some(DEFENSIVE)
    } else {
        ability_just_pressed(input)
    }) else {
        return;
    };

    cmds.next_anim_from_lookup(player_id, ability_to_anim_lookup(action), Some(weap_id));

    if action == DEFENSIVE {
        cmds.set_anim_hold(player_id, AnimationType::Block, true, Some(weap_id));
    } else {
        cmds.set_anim_hold(player_id, AnimationType::Block, false, Some(weap_id));
    }

    let Some(comb_cmd) = cmds.combat.iter().find(|c| c.entity_id == player_id) else {
        return;
    };

    ctrl.combat_state = Some(comb_cmd.requested_state);
}
