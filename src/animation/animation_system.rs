use crate::{
    command_buffer::{AnimOp, CommandBuffer},
    entity_manager::EntityManager,
};

pub fn update(em: &mut EntityManager, cmds: &mut CommandBuffer, dt: f32) {
    let acmds = std::mem::take(&mut cmds.anim);

    for c in acmds {
        let Some(animator) = em.animators.get_mut(c.target) else {
            eprintln!("Tried to find an animator for an entity that did not have one...");
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
        }
    }

    for entry in em.skellingtons.iter_mut() {
        let animator = em.animators.get_mut(entry.key()).unwrap();
        let skellington = entry.value_mut();

        animator.update(skellington, dt);
    }
}
