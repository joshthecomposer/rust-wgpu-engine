use glam::{Quat, Vec3};

use crate::{
    command_buffer::{AnimOp, CombCmd, CommandBuffer, ImpulseKind, PartCmd, PartKind},
    entity_manager::EntityManager,
    enums_types::AnimationType,
};

pub fn update(em: &mut EntityManager, cmds: &mut CommandBuffer, dt: f32) {
    let acmds = std::mem::take(&mut cmds.anim);

    for c in acmds {
        let Some(animator) = em.animators.get_mut(c.target) else {
            eprintln!("Tried to find an animator for an entity that did not have one...");
            continue;
        };

        let Some(_next_anim) = animator.get_next_animation() else {
            eprintln!("Tried to find a next animation and failed...");
            continue;
        };

        let Some(curr_anim) = animator.get_current_animation() else {
            eprintln!("Tried to find a next animation and failed...");
            continue;
        };

        match c.op {
            AnimOp::ResetAttacks => {
                let Some(id) = c.weapon else {
                    eprintln!("Tried to find weapon ID but failed");
                    continue;
                };

                let Some(helper) = em.weapon_helper.get_mut(id) else {
                    eprintln!("Tried to find weapon from id {} but failed", id);
                    continue;
                };

                helper.basic_chain = helper.basic_chain_default.clone();
            }
            AnimOp::SetNextAnimation(anim) => animator.set_next_animation(anim),
            AnimOp::SetCurrentAnimation(anim) => animator.set_current_animation(anim),
            AnimOp::DoHold(anim) => {
                let Some(a) = animator.animations.get_mut(&anim) else {
                    eprintln!("Tried to find an animator for an entity that did not have one...");
                    continue;
                };

                a.do_hold = true;
            }
            AnimOp::StopHold(anim) => {
                let Some(a) = animator.animations.get_mut(&anim) else {
                    eprintln!("Tried to find an animator for an entity that did not have one...");
                    continue;
                };

                a.do_hold = false;
            }
            AnimOp::SetAnimFromString(action) => {
                let Some(id) = c.weapon else {
                    eprintln!("Tried to find weapon ID but failed");
                    continue;
                };

                let Some(helper) = em.weapon_helper.get_mut(id) else {
                    eprintln!(
                        "Tried to find weapon action helper from id {} but failed",
                        id
                    );
                    continue;
                };

                let Some(abilities) = em.weapon_abilities.get(id) else {
                    eprintln!("Tried to find abilities from id {} but failed", id);
                    continue;
                };

                let cano_anim_name = match action.as_str() {
                    "basic" => {
                        let cano_anim_name = helper.basic_chain.first().unwrap().clone();
                        helper.basic_chain.rotate_left(1);
                        cano_anim_name
                    }
                    "dash" => helper.dash.clone(),
                    "skill1" => {
                        let aid = abilities.q;
                        let conf = em.abilities_config.get(aid).unwrap();
                        let cano_anim_name = &conf.animation;

                        cano_anim_name.to_string()
                    }
                    "skill2" => {
                        let aid = abilities.e;
                        let conf = em.abilities_config.get(aid).unwrap();
                        let cano_anim_name = &conf.animation;

                        cano_anim_name.to_string()
                    }
                    "ultimate" => {
                        let aid = abilities.r;
                        let conf = em.abilities_config.get(aid).unwrap();
                        let cano_anim_name = &conf.animation;

                        cano_anim_name.to_string()
                    }
                    "block" => helper.block.clone(),
                    _ => panic!("{}", action),
                };

                animator.set_next_animation(AnimationType::from_str(&cano_anim_name).unwrap());
            }
        }
    }

    for entry in em.skellingtons.iter_mut() {
        let id = entry.key();
        let animator = em.animators.get_mut(entry.key()).unwrap();
        let skellington = entry.value_mut();

        animator.update(skellington, dt);

        let anim = animator.get_next_animation().unwrap();

        if let Some(active_range_list) = &anim.hurtbox_activation {
            for fa in active_range_list {
                if fa.segment_range.contains(&anim.current_segment.get()) {
                    if !fa.triggered.get() {
                        fa.triggered.set(true);
                    }
                } else {
                    fa.triggered.set(false);
                }
            }
        }

        for os in anim.one_shots.iter() {
            if anim.current_segment.get() == os.segment {
                if !os.triggered.get() {
                    let trans = em.transforms.get(id).unwrap();
                    os.triggered.set(true);
                    cmds.sound3d(os.sound_type.clone(), trans.position);
                    // TODO: have the needed particle defined in the entity config
                    cmds.particles.push(PartCmd {
                        name: "DesertStep".to_string(),
                        kind: PartKind::EntityOrigin(id),
                        direction: glam::Vec3::ONE,
                    });
                }
            } else {
                os.triggered.set(false);
            }
        }

        let local_delta = animator.root_motion_state.frame_root_delta;

        if local_delta != Vec3::ZERO {
            let trans = em.transforms.get_mut(id).unwrap();
            let world_delta = trans.rotation * local_delta;

            let vx = world_delta.x / dt;
            let vz = world_delta.z / dt;

            // During knockback, don't override physics velocity with root-motion.
            if !em.knockbacks.contains(id) {
                cmds.set_linvel(id, ImpulseKind::Action, Vec3::new(vx, 0.0, vz));
            }
        }
    }
}
