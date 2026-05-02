use glam::{Mat4, Vec3};
use rapier3d::prelude::RigidBodyType;

use crate::{
    animation::animator::{self, Animator},
    command_buffer::{
        CommandBuffer, LocoCmd, LocoIntent, LocoSpace, PartCmd, PartKind, SoundCmd, SoundKind,
    },
    entity_manager::EntityManager,
    enums_types::{AnimationType, EnemyController, LifeState, SoundType},
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

        let anim_name = animator.next_animation;

        let Some(ctrl) = em.enemy_controllers.get_mut(eid) else {
            continue;
        };

        // ==========================================
        // Evaluate death stuff
        // ==========================================

        let skellington = em.skellingtons.get(eid).unwrap();
        let trans = em.transforms.get(eid).unwrap();

        match ctrl.life_state {
            LifeState::Alive => {}
            LifeState::Dying => {
                let Some(t) = em.transforms.get(eid) else {
                    em.entity_trashcan.push(eid);
                    continue;
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

                em.entity_trashcan.push(eid);
                continue;
            }
            LifeState::Dead => {
                continue;
            }
        }

        dbg!(anim_name);

        if ctrl.took_damage
            && !matches!(
                anim_name,
                AnimationType::Basic1
                    | AnimationType::Basic2
                    | AnimationType::Basic3
                    | AnimationType::OSBasic1
                    | AnimationType::OSBasic2
                    | AnimationType::OSBasic3
            )
        {
            cmds.next_anim(eid, AnimationType::Stagger, None);
            continue;
        }

        let weap_id = em.active_items.get(eid).and_then(|w| w.right_hand);

        let desired_action = ctrl.desired_action;

        let leaving_block =
            ctrl.current_action == ActionKind::Block && desired_action != Some(ActionKind::Block);

        if leaving_block {
            cmds.set_anim_hold(eid, AnimationType::Block, false, weap_id);
        }

        let health = em.healths.get(eid).unwrap();

        let dying = *health <= 0.0;

        let can_switch_action = anim.can_interrupt() || leaving_block || dying;

        let next_action = if can_switch_action {
            desired_action
        } else {
            Some(ctrl.current_action)
        };

        match next_action {
            Some(ActionKind::Idle) => {
                cmds.next_anim(eid, AnimationType::Idle, weap_id);
                ctrl.current_action = ActionKind::Idle;
            }
            Some(ActionKind::ChasePlayer) => {
                if anim.can_interrupt() {
                    if let Some(pid) = player_id {
                        let ptrans = em.transforms.get(pid).unwrap();
                        let etrans = em.transforms.get(eid).unwrap();
                        em.destinations.insert(eid, ptrans.position);
                        cmds.next_anim(eid, AnimationType::Run, weap_id);
                        ctrl.current_action = ActionKind::ChasePlayer;
                        let intent =
                            LocoIntent::build_ai_loco_intent(etrans.position, ptrans.position);

                        if !intent.is_zero() {
                            cmds.loco.push(LocoCmd {
                                target: eid,
                                intent,
                                allow_trans: true,
                                allow_rot: true,
                                space: LocoSpace::World,
                            });
                        }
                    }
                }
            }
            Some(ActionKind::AttackPlayer) => {
                if anim.can_interrupt() {
                    cmds.next_anim_from_lookup(eid, "basic".to_string(), weap_id);
                    ctrl.current_action = ActionKind::AttackPlayer;
                }
            }
            Some(ActionKind::Block) => {
                ctrl.current_action = ActionKind::Block;

                if animator.next_animation != AnimationType::Block && anim.can_interrupt() {
                    cmds.next_anim(eid, AnimationType::Block, weap_id);
                }

                if let Some(block_anim) = animator.animations.get(&AnimationType::Block) {
                    match block_anim.hold_frame {
                        Some(hold_frame) => {
                            if block_anim.current_segment.get() >= hold_frame {
                                cmds.set_anim_hold(eid, AnimationType::Block, true, weap_id);
                            }
                        }
                        None => {
                            cmds.set_anim_hold(eid, AnimationType::Block, true, weap_id);
                        }
                    }
                }
            }
            Some(ActionKind::Dodge) => {
                if anim.can_interrupt() {
                    cmds.next_anim_from_lookup(eid, "dash".to_string(), weap_id);
                    ctrl.current_action = ActionKind::Dodge;
                }
            }
            None => {}
        }
    }
}
