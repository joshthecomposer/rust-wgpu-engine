use crate::{
    abilities::{AbilitiesConfig, WeaponAbilities},
    animation::{animation::Animation, animator::Animator},
    command_buffer::{CommandBuffer, LocoIntent, LocoSpace},
    enums_types::{
        AnimationType, BufferedAction, CombatState, ControlState, LocoState, PlayerController,
        ANIMATION_EPSILON,
    },
    input::InputState,
    state_machines::player::loco::{
        ability_to_anim_lookup, ability_to_state, try_trigger_ability_cooldown,
    },
    util::constants::{BASIC, DEFENSIVE, EVADE, SKILL1, SKILL2, ULTIMATE},
};

pub fn combat_state_machine(
    ctrl: &mut PlayerController,
    cmds: &mut CommandBuffer,
    player_id: usize,
    weap_id: usize,
    weapon_abilities: &mut WeaponAbilities,
    abilities_config: &AbilitiesConfig,
    input: &InputState,
    animator: &mut Animator,
    dt: f32,
) {
    let Some(combat_state) = ctrl.combat_state else {
        eprintln!("combat_state_machine called with no combat_state");
        return;
    };

    let Some(anim) = animator.get_next_animation() else {
        eprintln!("Anim not found!");
        return;
    };

    let anim_name = animator.next_animation;

    let action = ctrl.queued_action;
    let intent = LocoIntent::build_loco_intent(input);

    ctrl.combat_time += dt;

    if combat_state == CombatState::Defensive {
        update_block_hold(cmds, player_id, weap_id, input, animator);
    }

    if anim.can_interrupt() {
        if try_consume_buffered_combat_action(
            ctrl,
            cmds,
            player_id,
            weap_id,
            weapon_abilities,
            abilities_config,
            action,
        ) {
            return;
        }

        if try_exit_combat_to_loco(ctrl, cmds, player_id, weap_id, &intent) {
            return;
        }
    }

    if try_reset_to_loco(player_id, weap_id, anim, ctrl, cmds) {
        return;
    } else {
        if anim_name == AnimationType::Spin2Win {
            cmds.loco.push(crate::command_buffer::LocoCmd {
                target: player_id,
                intent,
                allow_trans: true,
                allow_rot: true,
                space: LocoSpace::Camera,
            });
            return;
        }

        cmds.loco.push(crate::command_buffer::LocoCmd {
            target: player_id,
            intent,
            allow_trans: false,
            allow_rot: true,
            space: LocoSpace::Camera,
        });
    }
}

fn try_consume_buffered_combat_action(
    ctrl: &mut PlayerController,
    cmds: &mut CommandBuffer,
    player_id: usize,
    weap_id: usize,
    weapon_abilities: &mut WeaponAbilities,
    abilities_config: &AbilitiesConfig,
    action: Option<BufferedAction>,
) -> bool {
    let Some(buf) = action else {
        return false;
    };

    match buf.action {
        BASIC | EVADE | SKILL1 | SKILL2 | ULTIMATE | DEFENSIVE => {
            if !try_trigger_ability_cooldown(buf.action, weapon_abilities, abilities_config) {
                ctrl.queued_action = None;
                return false;
            }

            cmds.next_anim_from_lookup(
                player_id,
                ability_to_anim_lookup(buf.action),
                Some(weap_id),
            );
            ctrl.combat_state = Some(ability_to_state(buf.action));
            ctrl.queued_action = None;
            true
        }
        _ => {
            ctrl.queued_action = None;
            false
        }
    }
}

fn try_exit_combat_to_loco(
    ctrl: &mut PlayerController,
    cmds: &mut CommandBuffer,
    player_id: usize,
    weap_id: usize,
    intent: &LocoIntent,
) -> bool {
    if intent.is_zero() {
        return false;
    }

    cmds.next_anim(player_id, AnimationType::Run, None);
    ctrl.loco_state = LocoState::Running;
    ctrl.combat_state = None;
    ctrl.combat_time = 0.0;
    ctrl.control_state = ControlState::Player;
    ctrl.particle_cmd_issued = false;
    cmds.reset_attacks(player_id, Some(weap_id));
    true
}

fn update_block_hold(
    cmds: &mut CommandBuffer,
    player_id: usize,
    weap_id: usize,
    input: &InputState,
    animator: &Animator,
) {
    if !input.right_mouse_is_down() {
        cmds.set_anim_hold(player_id, AnimationType::Block, false, Some(weap_id));
        return;
    }

    let Some(block_anim) = animator.animations.get(&AnimationType::Block) else {
        eprintln!("Block animation missing!");
        return;
    };

    match block_anim.hold_frame {
        Some(hold_frame) => {
            if block_anim.current_segment.get() >= hold_frame {
                cmds.set_anim_hold(player_id, AnimationType::Block, true, Some(weap_id));
            }
        }
        None => {
            cmds.set_anim_hold(player_id, AnimationType::Block, true, Some(weap_id));
        }
    }
}

pub fn try_reset_to_loco(
    player_id: usize,
    weap_id: usize,
    anim: &Animation,
    ctrl: &mut PlayerController,
    cmds: &mut CommandBuffer,
) -> bool {
    if anim.current_time >= anim.duration - ANIMATION_EPSILON {
        cmds.next_anim(player_id, AnimationType::Idle, None);
        ctrl.loco_state = LocoState::Idle;
        ctrl.combat_state = None;
        ctrl.combat_time = 0.0;
        ctrl.control_state = ControlState::Player;
        ctrl.particle_cmd_issued = false;
        cmds.reset_attacks(player_id, Some(weap_id));
        true
    } else {
        false
    }
}
