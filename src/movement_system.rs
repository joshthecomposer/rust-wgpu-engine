use std::collections::HashSet;

use glam::{vec3, Quat, Vec3};
use rapier3d::{control::{CharacterAutostep, CharacterLength, KinematicCharacterController}, prelude::{QueryFilter, QueryFilterFlags}};

use crate::{camera::{CamMoveBasis, Camera}, entity_manager::{glam_to_nalgebra_quat, EntityManager}, enums_types::{AnimationType, CameraState, EntityType, Faction, PlayerState, SimState, Transform, VisualEffect, ANIMATION_EPSILON}, input::InputState, physics::PhysicsState, some_data::{FREEFALL_DELAY, GRAVITY}, terrain::Terrain};

pub fn update(
    em: &mut EntityManager, 
    dt: f32, 
    cam_basis: &CamMoveBasis, 
    input_state: &InputState, 
    ps: &mut PhysicsState
) {
    let player_keys = em.get_ids_for_faction(Faction::Player);

    handle_player_movement_rapier(input_state, em, player_keys, dt, cam_basis, ps);

    let enemy_keys = em.get_ids_for_faction(Faction::Enemy);

    handle_enemy_movement_rapier(enemy_keys, em, dt, ps);
}

fn handle_player_movement_rapier(
    input_state: &InputState,
    em: &mut EntityManager,
    player_keys: Vec<usize>,
    delta: f32,
    cam_basis: &CamMoveBasis,
    ps: &mut PhysicsState,
) {
    let player_key = *player_keys.first().unwrap();
    let animator = em.animators.get_mut(player_key).unwrap();
    let player_state = em.player_controllers.get(player_key).unwrap();
    let speed = em.base_speeds.get(player_key).unwrap();

    let kb = em.knockbacks.get_mut(player_key);

    let kb_active = em.knockbacks.get_mut(player_key).map_or(false, |kb| {
        kb.ttl -= delta;
        kb.ttl > 0.0
    });

    if kb_active { return; } else { em.knockbacks.remove(player_key); }

    if player_state.state == PlayerState::Dashing  { return; }

    if animator.next_animation == AnimationType::Death {
        return;
    }

    if player_state.state == PlayerState::Combat {
        return;
    }

    if player_state.state == PlayerState::Block {
        return;
    }

    em.v_effects.remove(player_key);

    let physics_handle = em.physics_handles.get(player_key).unwrap();
    let rb = ps.rigid_body_set.get_mut(physics_handle.rigid_body).unwrap();

    let mut move_dir = vec3(0.0, 0.0, 0.0);

    let forward_flat = vec3(cam_basis.fwd_flat.x, 0.0, cam_basis.fwd_flat.z).normalize_or_zero();
    let right_flat = vec3(cam_basis.right_flat.x, 0.0, cam_basis.right_flat.z).normalize_or_zero();

    if input_state.keys_current.contains(&glfw::Key::W) { move_dir += forward_flat; }
    if input_state.keys_current.contains(&glfw::Key::S) { move_dir -= forward_flat; }
    if input_state.keys_current.contains(&glfw::Key::D) { move_dir += right_flat; }
    if input_state.keys_current.contains(&glfw::Key::A) { move_dir -= right_flat; }

    let rotator = em.rotators.get_mut(player_key).unwrap();

    let mut linvel = *rb.linvel();


    if move_dir.length_squared() > 0.0 {
        let move_dir = move_dir.normalize();
        linvel.x = move_dir.x * speed;
        linvel.z = move_dir.z * speed;

        let yaw = f32::atan2(move_dir.x, move_dir.z);
        em.yaws.insert(player_key, yaw);

        let desired_rot = Quat::from_rotation_y(yaw); // * transform.original_rotation;

        if rotator.blend_factor == 0.0 && rotator.cur_rot != desired_rot {
            rotator.next_rot = desired_rot;
        }
    } else {
        linvel.x = 0.0;
        linvel.z = 0.0;
    };

    if rotator.next_rot != rotator.cur_rot {
        rotator.blend_factor += delta / rotator.blend_time;
        if rotator.blend_factor >= 1.0 {
            rotator.blend_factor = 0.0;
            rotator.cur_rot = rotator.next_rot;
        }
    }

    let smoothed = rotator.cur_rot.slerp(rotator.next_rot, rotator.blend_factor);
    rb.set_rotation(glam_to_nalgebra_quat(smoothed), true);
    rb.set_linvel(linvel, true);
}

fn handle_enemy_movement_rapier(
    ids: Vec<usize>,
    em: &mut EntityManager,
    dt: f32,
    ps: &mut PhysicsState,
) {
    for id in ids {

        let kb_active = em.knockbacks.get_mut(id).map_or(false, |kb| {
            kb.ttl -= dt;
            kb.ttl > 0.0
        });

        if kb_active { continue; } else { em.knockbacks.remove(id); }

        let Some(dest) = em.destinations.get(id) else { continue };

        let (
            Some(rotator),
            Some(physics_handle),
            Some(animator),
            Some(ent_type),
            Some(sim_controller),
        ) = (
            em.rotators.get_mut(id),
            em.physics_handles.get(id),
            em.animators.get_mut(id),
            em.entity_types.get(id),
            em.simstate_controllers.get(id),
        ) else { continue };

        let speed = match em.base_speeds.get(id) {
            Some(speed) => *speed,
            None => 1.5,
        };


        // TODO: Why god
        if animator.next_animation == AnimationType::Death
            || sim_controller.state == SimState::Dying 
            || sim_controller.state == SimState::Dead { continue };

        if animator.next_animation == AnimationType::Flinch { continue };

        if sim_controller.state == SimState::Combat { continue };

        em.v_effects.remove(id);

        if *ent_type == "MooseMan" { continue };

        let rb = ps.rigid_body_set.get_mut(physics_handle.rigid_body).unwrap();
        let position = Vec3::from_slice(rb.translation().as_slice());
        let direction = *dest - position;
        let distance = direction.length();

        if distance > 0.05 {
            let move_dir = direction.normalize();
            let velocity = move_dir * speed;

            // Set velocity
            let mut linvel = *rb.linvel();
            linvel.x = velocity.x;
            linvel.z = velocity.z;
            rb.set_linvel(linvel, true);

            // Set rotation
            let angle = f32::atan2(move_dir.x, move_dir.z);
            em.yaws.insert(id, angle);

            let desired_rot = Quat::from_rotation_y(angle);

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

            let blended = rotator.cur_rot.slerp(rotator.next_rot, rotator.blend_factor);
            rb.set_rotation(glam_to_nalgebra_quat(blended), true);

        } else {
            // Stop
            let mut linvel = *rb.linvel();
            linvel.x = 0.0;
            linvel.z = 0.0;
            rb.set_linvel(linvel, true);
        }

        // Sync Transform for rendering
        if let Some(transform) = em.transforms.get_mut(id) {
            let iso = rb.position();
            transform.position = Vec3::from_slice(iso.translation.vector.as_slice());
            transform.rotation = Quat::from_array(iso.rotation.coords.as_slice().try_into().unwrap());
        }
    }
}
