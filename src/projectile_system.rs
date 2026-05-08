use glam::vec3;

use crate::{
    command_buffer::{CommandBuffer, SoundCmd, SoundKind},
    entity_manager::EntityManager,
    enums_types::AnimationType,
    physics::PhysicsState,
};

pub struct ProjectileController {
    pub just_shot: bool,
}

impl ProjectileController {
    pub fn new() -> Self {
        Self { just_shot: false }
    }
}

pub fn update(em: &mut EntityManager, cmds: &mut CommandBuffer, ps: &mut PhysicsState) {
    spawn_projectiles(em, cmds, ps);
}

fn spawn_projectiles(em: &mut EntityManager, cmds: &mut CommandBuffer, ps: &mut PhysicsState) {
    // get active item ids, we don't care about anyone else in these cases.
    let ids = em
        .active_items
        .iter()
        .map(|ai| ai.value().right_hand.unwrap())
        .collect::<Vec<usize>>();

    for id in ids {
        let Some(parent_id) = em.owners.get(id).copied() else {
            eprintln!("Didn't find parent id?");
            continue;
        };

        let animator = em.animators.get(parent_id).unwrap();
        let anim = animator.get_next_animation().unwrap();

        let Some(pf) = anim.projectile_frame else {
            continue;
        };

        let Some(p_ctrl) = em.projectile_controllers.get_mut(parent_id) else {
            continue;
        };

        let on_projectile_frame = anim.current_segment.get() == pf;

        let should_spawn = on_projectile_frame && !p_ctrl.just_shot;

        p_ctrl.just_shot = on_projectile_frame;

        if !should_spawn {
            continue;
        }

        let Some(sphere_id) = em.create_sphere_projectile_from_weapon(parent_id, id, ps) else {
            continue;
        };

        let yaw = em.yaws.get(parent_id).unwrap();
        // direction * something
        let v = vec3(yaw.sin() * 50.0, 3.0, yaw.cos() * 50.0);

        cmds.impulse(
            sphere_id,
            Some(parent_id),
            crate::command_buffer::ImpulseKind::World,
            v,
        );

        cmds.sound3d_continuous(crate::enums_types::SoundType::Whee, sphere_id);
    }
}
