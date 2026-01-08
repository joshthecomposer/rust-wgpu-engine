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
    state_machines::player::orchestrator::{ability_just_pressed, anim_for_combat_state},
    util::constants::{BASIC, DEFENSIVE, EVADE, SKILL1, SKILL2, ULTIMATE},
};

pub fn combat_state_machine(
    ctrl: &mut PlayerController,
    cmds: &mut CommandBuffer,
    player_id: usize,
    weap_id: usize,
    input: &InputState,
    animator: &mut Animator,
    dt: f32,
) {
    let action = {
        if input.right_mouse_is_down() {
            Some(DEFENSIVE)
        } else {
            ability_just_pressed(input)
        }
    };

    let combat_state = ctrl.combat_state.unwrap();

    let anim = animator
        .animations
        .get(&anim_for_combat_state(&Some(combat_state)).unwrap())
        .unwrap();

    let can_interrupt = if let Some(f) = anim.interrupt_frame {
        if anim.current_segment.get() >= f {
            true
        } else {
            false
        }
    } else {
        true
    };

    let intent = LocoIntent::build_loco_intent(input);

    'a: {
        match combat_state {
            CombatState::Basic1 => {
                ctrl.combat_time += dt;
                if ctrl.combat_time >= 0.09 && anim.current_segment.get() < 15 {
                    ctrl.combat_time = 0.0;
                    cmds.particles.push(PartCmd {
                        name: "FireBallSwing".to_string(),
                        kind: PartKind::WeaponOrigin(weap_id),
                        direction: vec3(0.0, 1.0, 0.0),
                    });
                }
                if !ctrl.particle_cmd_issued && anim.current_segment.get() >= 15 {
                    cmds.particles.push(PartCmd {
                        name: "FireballMuzzleSpark".to_string(),
                        kind: PartKind::WeaponOrigin(weap_id),
                        direction: vec3(0.0, 1.0, 0.0),
                    });
                    cmds.particles.push(PartCmd {
                        name: "FireballMuzzleSmoke".to_string(),
                        kind: PartKind::WeaponOrigin(weap_id),
                        direction: vec3(0.0, 1.0, 0.0),
                    });
                    ctrl.particle_cmd_issued = true;
                }
                if can_interrupt {
                    match action {
                        Some(BASIC) => {
                            cmds.next_anim(player_id, AnimationType::Basic2, Some(weap_id));
                            ctrl.combat_state = Some(CombatState::Basic2);
                            ctrl.particle_cmd_issued = false;
                            break 'a;
                        }
                        Some(EVADE) => {
                            cmds.next_anim(player_id, AnimationType::DashF, Some(weap_id));
                            ctrl.combat_state = Some(CombatState::Evade);
                            ctrl.particle_cmd_issued = false;
                            break 'a;
                        }
                        Some(DEFENSIVE) => {
                            cmds.next_anim(player_id, AnimationType::Block, Some(weap_id));
                            ctrl.combat_state = Some(CombatState::Defensive);
                            ctrl.particle_cmd_issued = false;
                            break 'a;
                        }
                        _ => (),
                    }

                    if !intent.is_zero() {
                        cmds.next_anim(player_id, AnimationType::Run, None);
                        ctrl.loco_state = LocoState::Running;
                        ctrl.combat_state = None;
                        ctrl.control_state = ControlState::Player;
                        ctrl.particle_cmd_issued = false;
                        break 'a;
                    }
                }

                if anim.current_time >= anim.duration - ANIMATION_EPSILON {
                    cmds.next_anim(player_id, AnimationType::Idle, None);
                    ctrl.loco_state = LocoState::Idle;
                    ctrl.combat_state = None;
                    ctrl.control_state = ControlState::Player;
                    ctrl.particle_cmd_issued = false;
                    break 'a;
                }
            }
            CombatState::Basic2 => {
                ctrl.combat_time += dt;
                if ctrl.combat_time >= 0.09 && anim.current_segment.get() < 15 {
                    ctrl.combat_time = 0.0;
                    cmds.particles.push(PartCmd {
                        name: "FireBallSwing".to_string(),
                        kind: PartKind::WeaponOrigin(weap_id),
                        direction: vec3(0.0, 1.0, 0.0),
                    });
                }
                if !ctrl.particle_cmd_issued && anim.current_segment.get() >= 15 {
                    cmds.particles.push(PartCmd {
                        name: "FireballMuzzleSpark".to_string(),
                        kind: PartKind::WeaponOrigin(weap_id),
                        direction: vec3(0.0, 1.0, 0.0),
                    });
                    cmds.particles.push(PartCmd {
                        name: "FireballMuzzleSmoke".to_string(),
                        kind: PartKind::WeaponOrigin(weap_id),
                        direction: vec3(0.0, 1.0, 0.0),
                    });
                    ctrl.particle_cmd_issued = true;
                }
                if can_interrupt {
                    match action {
                        Some(BASIC) => {
                            cmds.next_anim(player_id, AnimationType::Basic3, Some(weap_id));
                            ctrl.combat_state = Some(CombatState::Basic3);
                            ctrl.particle_cmd_issued = false;
                            break 'a;
                        }
                        Some(EVADE) => {
                            cmds.next_anim(player_id, AnimationType::DashF, Some(weap_id));
                            ctrl.combat_state = Some(CombatState::Evade);
                            ctrl.particle_cmd_issued = false;
                            break 'a;
                        }
                        Some(DEFENSIVE) => {
                            cmds.next_anim(player_id, AnimationType::Block, Some(weap_id));
                            ctrl.combat_state = Some(CombatState::Defensive);
                            ctrl.particle_cmd_issued = false;
                            break 'a;
                        }
                        _ => (),
                    }

                    if !intent.is_zero() {
                        cmds.next_anim(player_id, AnimationType::Run, None);
                        ctrl.loco_state = LocoState::Running;
                        ctrl.combat_state = None;
                        ctrl.control_state = ControlState::Player;
                        ctrl.particle_cmd_issued = false;
                        break 'a;
                    }
                }

                if anim.current_time >= anim.duration - ANIMATION_EPSILON {
                    cmds.next_anim(player_id, AnimationType::Idle, None);
                    ctrl.loco_state = LocoState::Idle;
                    ctrl.combat_state = None;
                    ctrl.control_state = ControlState::Player;
                    ctrl.particle_cmd_issued = false;
                    break 'a;
                }
            }
            CombatState::Basic3 => {
                ctrl.combat_time += dt;
                if ctrl.combat_time >= 0.09 && anim.current_segment.get() < 20 {
                    ctrl.combat_time = 0.0;
                    cmds.particles.push(PartCmd {
                        name: "FireBallSwing".to_string(),
                        kind: PartKind::WeaponOrigin(weap_id),
                        direction: vec3(0.0, 1.0, 0.0),
                    });
                }
                if !ctrl.particle_cmd_issued && anim.current_segment.get() >= 20 {
                    cmds.particles.push(PartCmd {
                        name: "FireballMuzzleSpark".to_string(),
                        kind: PartKind::WeaponOrigin(weap_id),
                        direction: vec3(0.0, 1.0, 0.0),
                    });
                    cmds.particles.push(PartCmd {
                        name: "FireballMuzzleSmoke".to_string(),
                        kind: PartKind::WeaponOrigin(weap_id),
                        direction: vec3(0.0, 1.0, 0.0),
                    });
                    ctrl.particle_cmd_issued = true;
                }
                if can_interrupt {
                    match action {
                        Some(BASIC) => {
                            cmds.next_anim(player_id, AnimationType::Basic1, Some(weap_id));
                            ctrl.combat_state = Some(CombatState::Basic1);
                            ctrl.particle_cmd_issued = false;
                            break 'a;
                        }
                        Some(EVADE) => {
                            cmds.next_anim(player_id, AnimationType::DashF, Some(weap_id));
                            ctrl.combat_state = Some(CombatState::Evade);
                            ctrl.particle_cmd_issued = false;
                            break 'a;
                        }
                        Some(DEFENSIVE) => {
                            cmds.next_anim(player_id, AnimationType::Block, Some(weap_id));
                            ctrl.combat_state = Some(CombatState::Defensive);
                            ctrl.particle_cmd_issued = false;
                            break 'a;
                        }
                        _ => (),
                    }

                    if !intent.is_zero() {
                        cmds.next_anim(player_id, AnimationType::Run, None);
                        ctrl.loco_state = LocoState::Running;
                        ctrl.combat_state = None;
                        ctrl.control_state = ControlState::Player;
                        ctrl.particle_cmd_issued = false;
                        break 'a;
                    }
                }

                if anim.current_time >= anim.duration - ANIMATION_EPSILON {
                    cmds.next_anim(player_id, AnimationType::Idle, None);
                    ctrl.loco_state = LocoState::Idle;
                    ctrl.combat_state = None;
                    ctrl.control_state = ControlState::Player;
                    ctrl.particle_cmd_issued = false;
                    break 'a;
                }
            }
            CombatState::Evade => {
                if can_interrupt {
                    match action {
                        Some(BASIC) => {
                            cmds.next_anim(player_id, AnimationType::Basic1, Some(weap_id));
                            ctrl.combat_state = Some(CombatState::Basic1);
                            break 'a;
                        }
                        Some(EVADE) => {
                            cmds.next_anim(player_id, AnimationType::DashF, Some(weap_id));
                            ctrl.combat_state = Some(CombatState::Evade);
                            break 'a;
                        }
                        Some(DEFENSIVE) => {
                            cmds.next_anim(player_id, AnimationType::Block, Some(weap_id));
                            ctrl.combat_state = Some(CombatState::Defensive);
                            break 'a;
                        }
                        _ => (),
                    }

                    if !intent.is_zero() {
                        cmds.next_anim(player_id, AnimationType::Run, None);
                        ctrl.loco_state = LocoState::Running;
                        ctrl.combat_state = None;
                        ctrl.control_state = ControlState::Player;
                        break 'a;
                    }
                }

                if anim.current_time >= anim.duration - ANIMATION_EPSILON {
                    cmds.next_anim(player_id, AnimationType::Idle, None);
                    ctrl.loco_state = LocoState::Idle;
                    ctrl.combat_state = None;
                    ctrl.control_state = ControlState::Player;
                    break 'a;
                }
            }
            CombatState::Defensive => {
                if can_interrupt {
                    match action {
                        Some(BASIC) => {
                            cmds.next_anim(player_id, AnimationType::Basic1, Some(weap_id));
                            ctrl.combat_state = Some(CombatState::Basic1);
                            break 'a;
                        }
                        Some(EVADE) => {
                            cmds.next_anim(player_id, AnimationType::DashF, Some(weap_id));
                            ctrl.combat_state = Some(CombatState::Evade);
                            break 'a;
                        }
                        Some(DEFENSIVE) => {
                            cmds.next_anim(player_id, AnimationType::Block, Some(weap_id));
                            ctrl.combat_state = Some(CombatState::Defensive);
                            break 'a;
                        }
                        _ => (),
                    }
                    if !intent.is_zero() {
                        cmds.next_anim(player_id, AnimationType::Run, None);
                        ctrl.loco_state = LocoState::Running;
                        ctrl.combat_state = None;
                        ctrl.control_state = ControlState::Player;
                        break 'a;
                    }
                }

                if input.right_mouse_is_down() {
                    let block_anim = animator.animations.get(&AnimationType::Block).unwrap();

                    if let Some(hold_frame) = block_anim.hold_frame {
                        if block_anim.current_segment.get() == hold_frame {
                            cmds.set_anim_hold(
                                player_id,
                                AnimationType::Block,
                                true,
                                Some(weap_id),
                            );
                            break 'a;
                        }
                    }

                    break 'a;
                }
                cmds.set_anim_hold(player_id, AnimationType::Block, false, Some(weap_id));

                if anim.current_time >= anim.duration - ANIMATION_EPSILON {
                    cmds.next_anim(player_id, AnimationType::Idle, None);
                    ctrl.loco_state = LocoState::Idle;
                    ctrl.combat_state = None;
                    ctrl.control_state = ControlState::Player;
                    break 'a;
                }
            }
            _ => println!("We shouldn't be here yet..."),
        }
    }
}
