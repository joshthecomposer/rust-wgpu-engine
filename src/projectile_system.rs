use glam::vec3;

use crate::{command_buffer::CommandBuffer, entity_manager::EntityManager, physics::PhysicsState};

pub struct ProjectileShot {}

pub fn update(em: &mut EntityManager, cmds: &mut CommandBuffer, ps: &mut PhysicsState) {
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

        let should_spawn = match em.next_anim_info(parent_id) {
            Some((_, anim)) => match anim.projectile_frame {
                Some(pf) => anim.current_segment.get() == pf,
                None => false,
            },
            None => {
                eprintln!("No animation stuffs for this guy, why?");
                continue;
            }
        };

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
    }
}
