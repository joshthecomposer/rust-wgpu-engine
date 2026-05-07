use crate::{
    command_buffer::{CommandBuffer, PartCmd, PartKind, SoundCmd, SoundKind},
    entity_manager::EntityManager,
    enums_types::{AnimationType, BufferedAction, ControlState, LifeState, SoundType},
    input::InputState,
    state_machines::player::{
        combat::combat_state_machine,
        loco::{ability_is_ready, locomotion_state_machine},
    },
    util::constants::{BASIC, DEFENSIVE, EVADE, SKILL1, SKILL2, ULTIMATE},
};

use glam::Vec3;
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

    let Some(animator) = em.animators.get_mut(player_id) else {
        eprintln!("Missing animator");
        return;
    };

    let anim_name = animator.next_animation;

    let Some(ctrl) = em.player_controllers.get_mut(player_id) else {
        return;
    };

    let weap_id = em.active_items.get(player_id).and_then(|w| w.right_hand);

    if let Some(buffered_action) = input.collect_combat_input() {
        let weapon_abilities = weap_id.and_then(|id| em.weapon_abilities.get(id));

        if ability_is_ready(buffered_action, weapon_abilities) {
            ctrl.queued_action = Some(BufferedAction {
                action: buffered_action,
                ttl: 0.21,
            });
        }
    };

    if let Some(buf) = &mut ctrl.queued_action {
        buf.ttl -= dt;
        if buf.ttl <= 0.0 {
            ctrl.queued_action = None;
        }
    }

    let skellington = em.skellingtons.get(player_id).unwrap();
    let trans = em.transforms.get(player_id).unwrap();

    match ctrl.life_state {
        LifeState::Alive => {}
        LifeState::Dying => {
            let Some(t) = em.transforms.get(player_id) else {
                em.entity_trashcan.push(player_id);
                return;
            };

            let entity_world =
                glam::Mat4::from_scale_rotation_translation(t.scale, t.rotation, t.position);

            // Walk the full bone tree, not just the root's direct children.
            let mut stack = Vec::new();

            for bone in &skellington.children {
                stack.push(bone);
            }

            cmds.sound.push(SoundCmd {
                kind: SoundKind::Sound3d(SoundType::Bloop, trans.position),
            });

            while let Some(bone) = stack.pop() {
                let bone_world = entity_world * bone.global_transform;
                let pos = bone_world.w_axis.truncate();

                cmds.particles.push(PartCmd {
                    name: "BodyPoof".to_string(),
                    kind: PartKind::WorldOrigin(pos),
                    direction: Vec3::Y,
                });

                for child in &bone.children {
                    stack.push(child);
                }
            }

            em.entity_trashcan.push(player_id);
            return;
        }
        LifeState::Dead => {
            return;
        }
    }

    if ctrl.took_damage
        && !matches!(
            anim_name,
            AnimationType::Basic1
                | AnimationType::Basic2
                | AnimationType::Basic3
                | AnimationType::OSBasic1
                | AnimationType::OSBasic2
                | AnimationType::OSBasic3
                | AnimationType::Roll
                | AnimationType::Spin2Win
                | AnimationType::Stabby
        )
    {
        cmds.next_anim(player_id, AnimationType::Stagger, None);
        return;
    }

    let Some(gs) = em.grounded_states.get(player_id) else {
        eprintln!("We need a grounded state for player to work right");
        return;
    };

    let trans = em.transforms.get(player_id).unwrap();
    let pos = trans.position;

    match ctrl.control_state {
        ControlState::Player => {
            let weapon_abilities = weap_id.and_then(|id| em.weapon_abilities.get_mut(id));

            locomotion_state_machine(
                ctrl,
                input,
                cmds,
                player_id,
                weap_id,
                weapon_abilities,
                &em.abilities_config,
                animator,
                dt,
                gs,
                pos,
            );
        }
        ControlState::Combat => {
            if let Some(weap_id) = weap_id {
                let Some(weapon_abilities) = em.weapon_abilities.get_mut(weap_id) else {
                    return;
                };

                combat_state_machine(
                    ctrl,
                    cmds,
                    player_id,
                    weap_id,
                    weapon_abilities,
                    &em.abilities_config,
                    input,
                    animator,
                    dt,
                );
            }
        }
        _ => (),
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
