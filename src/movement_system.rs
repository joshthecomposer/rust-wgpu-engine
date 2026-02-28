use glam::{vec3, Quat, Vec3};
use winit::keyboard::KeyCode;

use crate::{
    camera::CamMoveBasis,
    command_buffer::{CommandBuffer, ImpulseKind},
    entity_manager::{glam_to_nalgebra_quat, EntityManager},
    enums_types::{AnimationType, PlayerState, SimState},
    input::InputState,
    physics::PhysicsState,
};

pub fn update(
    em: &mut EntityManager,
    cam_basis: &CamMoveBasis,
    cmds: &mut CommandBuffer,
    phys: &PhysicsState,
    dt: f32,
) {
    handle_loco_cmds(em, cam_basis, cmds, phys, dt);
}

fn handle_loco_cmds(
    em: &mut EntityManager,
    cam_basis: &CamMoveBasis,
    cmds: &mut CommandBuffer,
    phys: &PhysicsState,
    dt: f32,
) {
    let loco_cmds = std::mem::take(&mut cmds.loco);

    for lc in loco_cmds {
        let e = lc.target;

        let Some(speed) = em.base_speeds.get(e) else {
            continue;
        };

        let forward_flat =
            vec3(cam_basis.fwd_flat.x, 0.0, cam_basis.fwd_flat.z).normalize_or_zero();
        let right_flat =
            vec3(cam_basis.right_flat.x, 0.0, cam_basis.right_flat.z).normalize_or_zero();
        let mut move_dir = right_flat * lc.intent.x + forward_flat * lc.intent.z;

        let cur_y = em
            .physics_handles
            .get(e)
            .and_then(|ph| {
                let rb = phys.rigid_body_set.get(ph.rigid_body).unwrap();
                Some(rb.linvel().y)
            })
            .unwrap_or(0.0);

        let Some(rotator) = em.rotators.get_mut(e) else {
            continue;
        };

        if move_dir.length_squared() > 0.0 {
            move_dir = move_dir.normalize();

            let linvel = Vec3::new(move_dir.x * speed, cur_y, move_dir.z * speed);
            cmds.set_linvel(e, ImpulseKind::Locomotion, linvel);

            let yaw = f32::atan2(move_dir.x, move_dir.z);
            em.yaws.insert(e, yaw);

            let desired_rot = Quat::from_rotation_y(yaw);

            if rotator.blend_factor == 0.0 && rotator.cur_rot != desired_rot {
                rotator.next_rot = desired_rot;
            }

            if rotator.next_rot != rotator.cur_rot {
                rotator.blend_factor += dt / rotator.blend_time;
                if rotator.blend_factor >= 1.0 {
                    rotator.blend_factor = 0.0;
                    rotator.cur_rot = rotator.next_rot;
                }
            }

            let smoothed = rotator
                .cur_rot
                .slerp(rotator.next_rot, rotator.blend_factor);
            cmds.set_rot(e, ImpulseKind::Locomotion, smoothed);
        } else {
            cmds.set_linvel(e, ImpulseKind::Locomotion, Vec3::new(0.0, cur_y, 0.0));
        }
    }
}
