use crate::{
    command_buffer::{AnimOp, CombCmd, CommandBuffer},
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

        let Some(next_anim) = animator.get_next_animation() else {
            eprintln!("Tried to find a next animation and failed...");
            continue;
        };

        let Some(curr_anim) = animator.get_current_animation() else {
            eprintln!("Tried to find a next animation and failed...");
            continue;
        };

        match c.op {
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
                if !curr_anim.can_interrupt() {
                    continue;
                }

                let Some(id) = c.weapon else {
                    eprintln!("Tried to find weapon ID but failed");
                    continue;
                };

                let t = em.entity_types.get(id).unwrap();

                let Some(helper) = em.weapon_anim_map.weapon_types.get_mut(t) else {
                    eprintln!("Tried to find weapon from type {} but failed", t);
                    continue;
                };

                let cano_anim_name = match action.as_str() {
                    "basic" => {
                        let cano_anim_name = helper.basic_chain.first().unwrap().clone();
                        helper.basic_chain.rotate_left(1);
                        cano_anim_name
                    }
                    "dash" => helper.dash.clone(),
                    "block" => helper.block.clone(),
                    _ => panic!("{}", action),
                };

                animator.set_next_animation(AnimationType::from_str(&cano_anim_name).unwrap());
            }
        }
    }

    for entry in em.skellingtons.iter_mut() {
        let animator = em.animators.get_mut(entry.key()).unwrap();
        let skellington = entry.value_mut();

        animator.update(skellington, dt);
    }
}
