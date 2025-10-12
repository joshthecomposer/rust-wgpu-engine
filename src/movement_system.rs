use std::collections::HashSet;

use glam::{vec3, Quat, Vec3};
use rapier3d::{control::{CharacterAutostep, CharacterLength, KinematicCharacterController}, prelude::{QueryFilter, QueryFilterFlags}};

use crate::{camera::Camera, entity_manager::{glam_to_nalgebra_quat, EntityManager}, enums_types::{AnimationType, CameraState, EntityType, Faction, PlayerState, SimState, Transform, VisualEffect, ANIMATION_EPSILON}, input::InputState, physics::PhysicsState, some_data::{FREEFALL_DELAY, GRAVITY}, terrain::Terrain};

pub fn update(
    em: &mut EntityManager, 
    dt: f32, 
    camera: &Camera, 
    input_state: &InputState, 
    ps: &mut PhysicsState
) {
    let player_keys = em.get_ids_for_faction(Faction::Player);

    handle_player_movement_rapier(input_state, em, player_keys, dt, camera, ps);
}

fn handle_player_movement_rapier(
    input_state: &InputState,
    em: &mut EntityManager,
    player_keys: Vec<usize>,
    delta: f32,
    camera: &Camera,
    ps: &mut PhysicsState,
) {
    let player_key = *player_keys.first().unwrap();
    let animator = em.animators.get_mut(player_key).unwrap();
    let player_state = em.player_controllers.get(player_key).unwrap();
    //let speed = em.base_speeds.get(player_key).unwrap();
    let speed = 8.0;

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

    em.v_effects.remove(player_key);

    let physics_handle = em.physics_handles.get(player_key).unwrap();
    let rb = ps.rigid_body_set.get_mut(physics_handle.rigid_body).unwrap();

    if matches!(player_state.state, PlayerState::Jumping | PlayerState::Freefalling) && rb.linvel().y.abs() > ANIMATION_EPSILON {
        let transform = em.transforms.get_mut(player_key).unwrap();
        let iso = rb.position();
        transform.position = Vec3::from_slice(iso.translation.vector.as_slice());
        transform.rotation = Quat::from_array(iso.rotation.coords.as_slice().try_into().unwrap());
        return;
    }

    let mut move_dir = vec3(0.0, 0.0, 0.0);

    let forward_flat = vec3(camera.forward.x, 0.0, camera.forward.z).normalize_or_zero();
    let right_flat = vec3(camera.right.x, 0.0, camera.right.z).normalize_or_zero();

    if input_state.keys_current.contains(&glfw::Key::W) { move_dir += forward_flat; }
    if input_state.keys_current.contains(&glfw::Key::S) { move_dir -= forward_flat; }
    if input_state.keys_current.contains(&glfw::Key::D) { move_dir += right_flat; }
    if input_state.keys_current.contains(&glfw::Key::A) { move_dir -= right_flat; }

    let transform = em.transforms.get_mut(player_key).unwrap();
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

        // AnimationType::Run
    } else {
        linvel.x = 0.0;
        linvel.z = 0.0;
        // AnimationType::Idle
    };

    // animator.next_animation = new_state;

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

fn handle_player_movement_rapier_kinematic(
    input_state: &InputState,
    em: &mut EntityManager,
    player_keys: Vec<usize>,
    dt: f32,
    camera: &Camera,
    ps: &mut PhysicsState,
) {
    let player_key = *player_keys.first().unwrap();

    let mut kcc = KinematicCharacterController::default();
    kcc.offset = CharacterLength::Absolute(0.01);
    kcc.slide = true;
    kcc.autostep = None;

    let speed = 20.0;
    
    let ph = em.physics_handles.get(player_key).unwrap();

    let forward_flat = glam::vec3(camera.forward.x, 0.0, camera.forward.z).normalize();
    let right_flat = glam::vec3(camera.right.x, 0.0, camera.right.z).normalize();

    let mut move_dir = glam::Vec3::ZERO;

    if input_state.keys_current.contains(&glfw::Key::W) { move_dir += forward_flat; }
    if input_state.keys_current.contains(&glfw::Key::S) { move_dir -= forward_flat; }
    if input_state.keys_current.contains(&glfw::Key::D) { move_dir += right_flat;   }
    if input_state.keys_current.contains(&glfw::Key::A) { move_dir -= right_flat;   }

    if move_dir.length_squared() > 0.0 { move_dir = move_dir.normalize(); }

    let mut desired = move_dir * speed * dt;
    desired.y += -GRAVITY * dt;

    let (body_iso, shape, local_iso) = {
        let rb  = ps.rigid_body_set.get(ph.rigid_body).unwrap();
        let col = ps.collider_set.get(ph.collider).unwrap();

        (*rb.position(), col.shape(), col.position_wrt_parent().unwrap())
    };

    let shape_iso = body_iso * local_iso;


    let filter = QueryFilter {
        flags: QueryFilterFlags::EXCLUDE_SENSORS,
        groups: None,
        exclude_collider: Some(ph.collider),
        exclude_rigid_body: Some(ph.rigid_body),
        predicate: None,
    };


    let output = kcc.move_shape(
        dt.into(),
        &ps.rigid_body_set,
        &ps.collider_set,
        ps.query_pipeline.as_ref().unwrap(),
        shape,
        &shape_iso,
        desired.into(),
        filter,
        |_| {},
    );

    {
        let rb_mut = ps.rigid_body_set.get_mut(ph.rigid_body).unwrap();
        let mut iso = *rb_mut.position();
        iso.translation.vector += output.translation;
        rb_mut.set_next_kinematic_position(iso);

    }

    let hitbox_entity = em.collider_to_parent.get(&ph.collider).unwrap();

    let rb_cur = ps.rigid_body_set.get(ph.rigid_body).unwrap();
    let iso = rb_cur.position();

    let t = em.transforms.get_mut(player_key).unwrap();

    t.position = glam::Vec3::from_slice(iso.translation.vector.as_slice());
    t.rotation = glam::Quat::from_array(iso.rotation.coords.as_slice().try_into().unwrap());

    let t = t.clone();

    let hbt = em.transforms.get_mut(*hitbox_entity).unwrap();
    hbt.position = t.position;
    hbt.rotation = t.rotation;
}
