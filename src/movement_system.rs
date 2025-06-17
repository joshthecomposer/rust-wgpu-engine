use std::collections::HashSet;

use glam::{vec3, Quat, Vec3};

use crate::{camera::Camera, entity_manager::EntityManager, enums_types::{AnimationType, CameraState, EntityType, Faction, Transform}, input::InputState, terrain::Terrain};

pub fn update(em: &mut EntityManager, terrain: &Terrain, dt: f32, camera: &Camera, input_state: &InputState) {
    
    let player_keys = em.get_ids_for_faction(Faction::Player);
    let enemy_keys = em.get_ids_for_faction(Faction::Enemy);
    let static_keys = em.get_ids_for_faction(Faction::Static);
    let gizmo_keys = em.get_ids_for_faction(Faction::Gizmo);

    if camera.move_state != CameraState::Free {
        if player_keys.len() > 0 {
            handle_player_movement(input_state, em, player_keys, dt, camera, terrain);
        }
    }
    handle_enemy_movement(enemy_keys, em, terrain, dt,);
    handle_static_movement(static_keys, em, terrain);
    handle_gizmo_movement(gizmo_keys, em, dt);
}

fn handle_player_movement(input_state: &InputState, em: &mut EntityManager, player_keys: Vec<usize>, delta: f32, camera: &Camera, terrain: &Terrain) {
    let player_key = *player_keys.first().unwrap();
    let animator = em.animators.get_mut(player_key).unwrap();

    if animator.next_animation == AnimationType::Death {
        return;
    }

    if input_state.keys_current.contains(&glfw::Key::T) {
        animator.set_next_animation(AnimationType::Slash);
        
        let anim = animator.animations.get_mut(&AnimationType::Slash).unwrap();
        if !input_state.keys_previous.contains(&glfw::Key::T) {
            anim.current_time = 0.0;
        }
    } else {
        animator.restarted = false;
        let speed = 5.0 * delta;
        let mut move_dir = vec3(0.0, 0.0, 0.0);

        let forward_flat = vec3(camera.forward.x, 0.0, camera.forward.z).normalize();
        let right_flat = vec3(camera.right.x, 0.0, camera.right.z).normalize();

        if input_state.keys_current.contains(&glfw::Key::W) {
            move_dir += forward_flat;
        }
        if input_state.keys_current.contains(&glfw::Key::S) {
            move_dir -= forward_flat;
        }
        if input_state.keys_current.contains(&glfw::Key::D) {
            move_dir += right_flat;
        }
        if input_state.keys_current.contains(&glfw::Key::A) {
            move_dir -= right_flat;
        }

        let mut velocity = vec3(0.0, 0.0, 0.0);
        let new_rotation: Option<Quat>;

        let new_state = if move_dir.length_squared() > 0.0 {
            move_dir = move_dir.normalize();
            velocity = move_dir * speed;

            let rot =Quat::from_rotation_y(f32::atan2(-move_dir.x, -move_dir.z));
            new_rotation = Some(rot * em.transforms.get(player_key).unwrap().original_rotation.normalize());
            AnimationType::Run 
        } else {
            new_rotation = None;
            AnimationType::Idle
        };

        let transform = em.transforms.get_mut(player_key).unwrap();
        let rotator = em.rotators.get_mut(player_key).unwrap();
        if rotator.next_rot != rotator.cur_rot {
            rotator.blend_factor += delta as f32 / rotator.blend_time;
            if rotator.blend_factor >= 1.0 {
                rotator.blend_factor = 0.0;
                rotator.cur_rot = rotator.next_rot;
            }
        }

        animator.next_animation = new_state;

        if let Some(rot) = new_rotation {
            if rotator.blend_factor == 0.0 && rot != rotator.cur_rot {
                rotator.next_rot = rot;
            }

            transform.rotation = rotator.cur_rot.slerp(rotator.next_rot, rotator.blend_factor);
        }

        // TODO: This should likely be different and calculated in the collision system
        transform.position.y = terrain.get_height_at(transform.position.x, transform.position.z);

        transform.position += velocity;

    }

}

fn handle_enemy_movement(ids: Vec<usize>, em: &mut EntityManager, terrain: &Terrain, dt: f32) {
    for id in ids {
        if let (
            Some(trans),
            Some(rotator),
        ) = (
            em.transforms.get_mut(id),
            em.rotators.get_mut(id),
        ) {
            trans.position.y = terrain.get_height_at(trans.position.x, trans.position.z);
            let speed = 3.2 * dt as f32;
            let destination = em.destinations.get(id).unwrap();
            let direction = *destination - trans.position;
            let distance = direction.length();

            if distance > 0.001 {
                // translation
                let calc_movement = direction.normalize() * speed.min(distance);

                trans.position += calc_movement;

                // Rotation
                let movement_dir = direction.normalize();
                // let up = Vec3::Y;


                // TODO: This clamps rotation to around Y, which should be not the case forever.
                let angle = f32::atan2(-movement_dir.x, -movement_dir.z);
                let target_rot = Quat::from_rotation_y(angle) * trans.original_rotation;

                // let target_rot = Quat::from_rotation_arc(-Vec3::Z, movement_dir) * trans.original_rotation;

                if rotator.blend_factor == 0.0 && target_rot != rotator.cur_rot {
                    rotator.next_rot = target_rot;
                }

                if rotator.next_rot != rotator.cur_rot {
                    rotator.blend_factor += dt / rotator.blend_time;
                    if rotator.blend_factor >= 1.0 {
                        rotator.blend_factor = 0.0;
                        rotator.cur_rot = rotator.next_rot;
                    }
                }

                trans.rotation = rotator.cur_rot.slerp(rotator.next_rot, rotator.blend_factor);
            }

        }
    }
}

fn handle_static_movement(ids: Vec<usize>, em: &mut EntityManager, terrain: &Terrain) {
    // TODO: This terrain adjustment should be in the collision system file.
    for id in ids {
        if let Some(ent_type) = em.entity_types.get(id) {
            if ent_type != &EntityType::Terrain {
                if let Some(trans) = em.transforms.get_mut(id) {
                    trans.position.y = terrain.get_height_at(trans.position.x, trans.position.z);
                }
            }
        }
    }
}

fn handle_gizmo_movement(ids: Vec<usize>, em: &mut EntityManager, dt: f32) {
    let mut transforms_to_update:Vec<(usize, usize)> = vec![];
    for id in ids {
        if let Some(parent) = em.parents.get(id) {
            transforms_to_update.push((id, parent.parent_id))
        }
    }

    for (child_id, parent_id) in transforms_to_update {
        let parent_transform = em.transforms.get(parent_id).unwrap().clone();
        let child_transform = em.transforms.get(child_id).unwrap().clone();

        // Some magic to make sure the cylinder is rotated properly despite the parent being originally offset in some way
        let adjusted_rotation = parent_transform.rotation
        * parent_transform.original_rotation.inverse()
        * child_transform.original_rotation.inverse();

        em.transforms.insert(child_id, Transform {
            position: parent_transform.position,
            rotation: adjusted_rotation,
            scale: child_transform.scale,
            original_rotation: child_transform.original_rotation,
        });
    }
}

fn revolve_around_something(object: &mut Vec3, target: &Vec3, elapsed: f32, radius: f32, speed: f32) {
    let angle = elapsed * speed;

    object.x = target.x + radius * angle.cos();
    object.z = target.z + radius * angle.sin();
    object.y = target.y + 1.0;
}

