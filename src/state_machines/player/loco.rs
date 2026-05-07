use glam::{vec3, Vec3};

use crate::{
    abilities::{AbilitiesConfig, WeaponAbilities},
    animation::animator::Animator,
    command_buffer::{CommandBuffer, LocoCmd, LocoIntent, LocoSpace, PartCmd, PartKind},
    enums_types::{
        AnimationType, CombatState, ControlState, GroundedState, LocoState, PlayerController,
    },
    input::InputState,
    state_machines::player::orchestrator::ability_just_pressed,
    util::constants::{BASIC, DEFENSIVE, EVADE, SKILL1, SKILL2, ULTIMATE},
};

pub fn locomotion_state_machine(
    ctrl: &mut PlayerController,
    input: &InputState,
    cmds: &mut CommandBuffer,
    player_id: usize,
    weap_id: Option<usize>,
    mut weapon_abilities: Option<&mut WeaponAbilities>,
    abilities_config: &AbilitiesConfig,
    animator: &Animator,
    dt: f32,
    gs: &GroundedState,
    pos: Vec3,
) {
    // Are we locked in combat?
    if !ctrl.can_loco() {
        return;
    }

    let intent = LocoIntent::build_loco_intent(input);

    ctrl.combat_state = None;
    ctrl.control_state = ControlState::Player;

    'a: {
        match ctrl.loco_state {
            LocoState::Init => {
                cmds.next_anim(player_id, AnimationType::Idle, None);
                ctrl.loco_state = LocoState::Idle;
            }
            LocoState::Idle => {
                // Go to combat?
                if let (Some(ability), Some(weap_id), Some(weapon_abilities)) = (
                    ability_just_pressed(input),
                    weap_id,
                    weapon_abilities.as_deref_mut(),
                ) {
                    if transition_to_combat(
                        player_id,
                        ctrl,
                        cmds,
                        ability,
                        weap_id,
                        weapon_abilities,
                        abilities_config,
                    ) {
                        break 'a;
                    }
                }

                check_new_loco(intent, input, ctrl, cmds, player_id, animator);
            }
            LocoState::Running => {
                if let (Some(ability), Some(weap_id), Some(weapon_abilities)) = (
                    ability_just_pressed(input),
                    weap_id,
                    weapon_abilities.as_deref_mut(),
                ) {
                    if transition_to_combat(
                        player_id,
                        ctrl,
                        cmds,
                        ability,
                        weap_id,
                        weapon_abilities,
                        abilities_config,
                    ) {
                        break 'a;
                    }
                }

                check_new_loco(intent, input, ctrl, cmds, player_id, animator);
            }
            LocoState::Jumping => {
                let jump_anim = animator.animations.get(&AnimationType::Jump).unwrap();

                if jump_anim.current_segment.get() >= 8 && !ctrl.jump_command_issued {
                    cmds.jump(player_id);
                    cmds.loco.push(LocoCmd {
                        target: player_id,
                        intent,
                        allow_trans: true,
                        allow_rot: true,
                        space: LocoSpace::Camera,
                    });
                    ctrl.jump_command_issued = true;
                }

                if gs.just_left {
                    loco_transition(player_id, ctrl, cmds, LocoState::Airborne, intent, animator);
                    ctrl.jump_command_issued = false;
                }
            }
            LocoState::Airborne => {
                ctrl.loco_time += dt;

                cmds.loco.push(LocoCmd {
                    target: player_id,
                    intent,
                    allow_trans: false,
                    allow_rot: true,
                    space: LocoSpace::Camera,
                });

                let jump_anim = animator.animations.get(&AnimationType::Jump).unwrap();

                if gs.just_landed || (gs.is_grounded && ctrl.loco_time >= 0.4) {
                    loco_transition(player_id, ctrl, cmds, LocoState::Running, intent, animator);
                    cmds.particles.push(PartCmd {
                        name: "DesertLand".to_string(),
                        direction: vec3(0.0, 1.0, 0.0),
                        kind: PartKind::WorldOrigin(pos),
                    });
                    cmds.sound3d(pos);
                    break 'a;
                }

                if jump_anim.current_segment.get() >= 15
                    && animator.next_animation != AnimationType::Freefall
                {
                    cmds.next_anim(player_id, AnimationType::Freefall, None);
                    break 'a;
                }
            }
            _ => println!("this shouldn't have happened dog"),
        }
    }
}

// Should we do a new locomotion state?
pub fn check_new_loco(
    intent: LocoIntent,
    input: &InputState,
    ctrl: &mut PlayerController,
    cmds: &mut CommandBuffer,
    player_id: usize,
    animator: &Animator,
) {
    if input.space_just_pressed() {
        loco_transition(player_id, ctrl, cmds, LocoState::Jumping, intent, animator);
        return;
    }

    if !intent.is_zero() {
        loco_transition(player_id, ctrl, cmds, LocoState::Running, intent, animator);
        return;
    }

    if intent.is_zero() {
        loco_transition(player_id, ctrl, cmds, LocoState::Idle, intent, animator);
        return;
    }
}

pub fn loco_transition(
    player_id: usize,
    ctrl: &mut PlayerController,
    cmds: &mut CommandBuffer,
    state: LocoState,
    intent: LocoIntent,
    animator: &Animator,
) {
    if state == LocoState::Jumping {
        ctrl.loco_time = 0.0;
        ctrl.loco_state = state;
        cmds.next_anim(player_id, AnimationType::Jump, None);
        return;
    }

    if state == LocoState::Airborne {
        ctrl.loco_time = 0.0;
        ctrl.loco_state = state;
        return;
    }

    ctrl.loco_state = state;
    ctrl.loco_time = 0.0;
    cmds.next_anim(player_id, anim_for_loco_state(&state), None);

    cmds.loco.push(LocoCmd {
        target: player_id,
        intent,
        allow_trans: true,
        allow_rot: true,
        space: LocoSpace::Camera,
    });
}

pub fn ability_to_anim_lookup(ability: u32) -> String {
    match ability {
        BASIC => "basic".to_string(),
        EVADE => "dash".to_string(),
        SKILL1 => "skill1".to_string(),
        SKILL2 => "skill2".to_string(),
        ULTIMATE => "ultimate".to_string(),
        DEFENSIVE => "block".to_string(),
        _ => "basic".to_string(),
    }
}

pub fn ability_to_slot(ability: u32) -> Option<usize> {
    match ability {
        SKILL1 => Some(2),
        SKILL2 => Some(3),
        ULTIMATE => Some(5),
        _ => None,
    }
}

pub fn ability_is_ready(ability: u32, weapon_abilities: Option<&WeaponAbilities>) -> bool {
    match ability_to_slot(ability) {
        Some(slot) => weapon_abilities.is_some_and(|abilities| abilities.is_ready(slot)),
        None => true,
    }
}

pub fn try_trigger_ability_cooldown(
    ability: u32,
    weapon_abilities: &mut WeaponAbilities,
    abilities_config: &AbilitiesConfig,
) -> bool {
    match ability_to_slot(ability) {
        Some(slot) => weapon_abilities.trigger(slot, abilities_config).is_some(),
        None => true,
    }
}

pub fn ability_to_state(ability: u32) -> CombatState {
    match ability {
        BASIC => CombatState::Basic,
        EVADE => CombatState::Evade,
        SKILL1 => CombatState::Skill1,
        SKILL2 => CombatState::Skill2,
        ULTIMATE => CombatState::Ultimate,
        DEFENSIVE => CombatState::Defensive,
        _ => CombatState::Basic,
    }
}

fn transition_to_combat(
    player_id: usize, // id
    ctrl: &mut PlayerController,
    cmds: &mut CommandBuffer,
    ability: u32,
    weap_id: usize,
    weapon_abilities: &mut WeaponAbilities,
    abilities_config: &AbilitiesConfig,
) -> bool {
    if !try_trigger_ability_cooldown(ability, weapon_abilities, abilities_config) {
        return false;
    }

    ctrl.control_state = ControlState::Combat;
    cmds.next_anim_from_lookup(player_id, ability_to_anim_lookup(ability), Some(weap_id));
    ctrl.combat_state = Some(ability_to_state(ability));
    true
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
