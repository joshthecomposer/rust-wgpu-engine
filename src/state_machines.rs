use glam::{vec3, Mat4, Vec3};
use glfw::{Key, MouseButton};

use crate::{entity_manager::EntityManager, enums_types::{AnimationType, AttackState, EntityType, Faction, PlayerState, SimState, SoundType, VisualEffect, ANIMATION_EPSILON}, input::InputState, particles::ParticleSystem, physics::PhysicsState, some_data::{DECREASED_GRAVITY_SCALAR, GRAVITY}, sound::sound_manager::SoundManager, util::data_structure::HashMapGetPairMut};

pub fn update(em: &mut EntityManager, dt: f32, particles: &mut ParticleSystem, input: &InputState, ps: &mut PhysicsState, sm: &mut SoundManager) {
    player_state_machine(em, dt, input, ps, sm);
    entity_sim_state_machine(em, dt, particles);
}

fn entity_sim_state_machine(em: &mut EntityManager, dt: f32, particles: &mut ParticleSystem) {
    for fac in em.factions.iter() {
        if *fac.value() == Faction::Enemy {
            let controller = em.simstate_controllers.get_mut(fac.key()).unwrap();
            let player_key = em.factions.iter().find(|e| *e.value() == Faction::Player).unwrap().key();
            let player_pos = em.transforms.get(player_key).unwrap().position;
            let entity_pos = em.transforms.get(fac.key()).unwrap().position;
            let animator = em.animators.get_mut(fac.key()).unwrap();
            let destination = em.destinations.get_mut(fac.key()).unwrap();
            let health = em.healths.get(fac.key()).unwrap();

            let active_weapon_id = em
                .active_items
                .get(fac.key())
                .and_then(|ai| ai.right_hand);

            let distance = active_weapon_id
                .and_then(|wid| {
                    em.parents
                        .iter()
                        .find(|p| p.value().parent_id == wid && em.cuboids.get(p.key()).is_some())
                        .and_then(|entry| em.cuboids.get(entry.key()).map(|hb| hb.h)) // child id = entry.key()
                })
                .unwrap_or(3.0); // fallback if no weapon or no cuboid

            let trans = em.transforms.get(fac.key()).unwrap();

            let next_state = (|| match controller.state {
                SimState::Dancing => {
                    *destination = entity_pos;
                    animator.set_next_animation(AnimationType::Dance);
                    SimState::Dancing
                },
                SimState::Waiting => {
                    if *health <= 0.0 { return SimState::Dying; }
                    animator.set_next_animation(AnimationType::Idle);
                    *destination = entity_pos;

                    let to_player = (player_pos - entity_pos).with_y(0.0).normalize();
                    // let forward = (trans.rotation * trans.original_rotation.inverse() * -Vec3::Z).with_y(0.0).normalize();
                    let forward = (trans.rotation * Vec3::Z).with_y(0.0).normalize();
                    let alignment = forward.dot(to_player);
                    let fov_threshold = 0.5; // cos(30 degrees);

                    let view_distance = 12.0;

                    let player_in_range = entity_pos.distance(player_pos) <= view_distance;

                    if  alignment >= fov_threshold && player_in_range {
                        return SimState::Aggro
                    }

                    SimState::Waiting
                },
                SimState::Aggro => {
                    if *health <= 0.0 { return SimState::Dying; }

                    controller.time_in_state += dt;
                    animator.set_next_animation(AnimationType::Run);
                    *destination = player_pos;

                    if entity_pos.distance(player_pos) < distance {
                        animator.set_next_animation(AnimationType::Slash);
                        controller.time_in_state = 0.0;
                        return SimState::Attacking
                    }

                    if entity_pos.distance(player_pos) > 12.0 {
                        return SimState::Waiting
                    } 


                    SimState::Aggro
                },
                SimState::Dying => {

                    em.v_effects.remove(fac.key());

                    animator.set_next_animation(AnimationType::Death);
                    *destination = entity_pos;
                    
                    if let Some(anim) = animator.animations.get(&AnimationType::Death) {
                        if anim.current_time >= anim.duration - ANIMATION_EPSILON {
                            return SimState::Dead { time: 0.0, target_time: 5.0 }
                        } 
                    } else {
                        // particles.spawn_oneshot_emitter(1000, entity_pos);
                        em.entity_trashcan.push(fac.key());
                    }
                    
                    SimState::Dying
                },
                SimState::Dead { time, target_time } => {
                    animator.set_next_animation(AnimationType::Death);

                    let new_time = time + dt;

                    if new_time >= 4.0 {
                        em.v_effects.insert(fac.key(), VisualEffect::Flashing);
                    }

                    if new_time >= target_time {
                        let model_transform = Mat4::from_scale_rotation_translation(trans.scale, trans.rotation, trans.position);
                        let skellington = em.skellingtons.get_mut(fac.key()).unwrap();

                        let bone_names: Vec<String> = {
                            let anim = animator.animations.get(&animator.current_animation).unwrap();
                            anim.model_animation_join.iter().map(|b| b.name.clone()).collect()
                        };

                        let anim = animator.animations.get_mut(&animator.current_animation).unwrap();

                        for bone_name in bone_names{
                            if let Some(bone_world_model_space) = anim.get_raw_global_bone_transform_by_name(
                                &bone_name,
                                skellington,
                                Mat4::IDENTITY,
                            ) {
                                let bone_world_space = model_transform * bone_world_model_space;
                                let position = bone_world_space.w_axis.truncate();

                                // You can randomize velocity or make it static for now
                                // particles.spawn_oneshot_emitter(100, position);
                            }
                        }


                        // let model = Mat4::from_scale_rotation_translation(trans.scale, trans.rotation, trans.position);
                        // let  anim = animator.animations.get_mut(&animator.current_animation).unwrap();
                        // let skellington = em.skellingtons.get(fac.key()).unwrap();

                        // if let Some(neck_transform_model_space) = anim.get_raw_global_bone_transform("mixamorig:Neck", skellington, Mat4::IDENTITY) {
                        //     let world_transform = model * neck_transform_model_space;
                        //     let neck_position = world_transform.w_axis.truncate();
                        //     particles.spawn_particles(1000, neck_position);
                        // }

                        // if let Some(hip_transform_model_space) = anim.get_raw_global_bone_transform("mixamorig:Hips", skellington, Mat4::IDENTITY) {
                        //     let world_transform = model * hip_transform_model_space;
                        //     let neck_position = world_transform.w_axis.truncate();
                        //     particles.spawn_particles(1000, neck_position);
                        // }
                        em.entity_trashcan.push(fac.key());
                    }

                    SimState::Dead { time: new_time, target_time: target_time }
                },
                SimState::Attacking => {
                    if *health <= 0.0 { return SimState::Dying; }

                    let anim = animator.animations.get_mut(&AnimationType::Slash).unwrap();

                    if anim.current_time >= anim.duration - ANIMATION_EPSILON {
                        controller.time_in_state = 0.0;
                        anim.current_time = 0.0;
                        return SimState::Aggro;
                    }

                    controller.time_in_state += dt;



                    return SimState::Attacking;
                },
            })();

            controller.state = next_state;
        }
    }
}

fn player_state_machine(em: &mut EntityManager, dt: f32, input: &InputState, ps: &mut PhysicsState, sm: &mut SoundManager){ 
    let player_key = em.factions.iter().find(|e| *e.value() == Faction::Player).unwrap().key();
    let controller = em.player_controllers.get_mut(player_key).unwrap();
    let ph = em.physics_handles.get(player_key).unwrap();
    let rb = ps.rigid_body_set.get_mut(ph.rigid_body).unwrap();
    let animator = em.animators.get_mut(player_key).unwrap();
    let trans = em.transforms.get(player_key).unwrap();

    let ground_id = em.entity_types.iter().find(|e| *e.value() == EntityType::Terrain).unwrap().key();
    let ground_ph = em.physics_handles.get(ground_id).unwrap();

    let yaw = em.yaws.get(player_key).unwrap();
    let dir = vec3(yaw.sin(), 1.0, yaw.cos()).normalize();

    let impulse_strength = vec3(7.0, 3.5, 7.0);
    let m = rb.mass();
    let impulse = vec3(dir.x * (7.0 * m), 0.0, dir.z * (7.0 * m));

    let next_state = (|| match controller.state {
        // ==================================================================================
        // PLAYER IDLE 
        // ==================================================================================
        PlayerState::Idle => {
            controller.time_in_state += dt;
            if input.just_pressed(Key::Space) {
                if input.is_down(Key::LeftShift) {
                    rb.apply_impulse(impulse.into(), true);

                    animator.set_next_animation(AnimationType::DashF);

                    return PlayerState::Dashing;
                }

                rb.set_gravity_scale(DECREASED_GRAVITY_SCALAR, true);
                rb.apply_impulse((Vec3::Y * 5.2).into(), true);
                
                if let Some(jump_anim) = animator.animations.get_mut(&AnimationType::Jump) {
                    jump_anim.current_time = 0.0;
                    animator.set_next_animation(AnimationType::Jump);
                }

                controller.time_in_state = 0.0;
                return PlayerState::Jumping
            }

            if input.mouse_just_pressed(MouseButton::Left) {
                animator.animations.get_mut(&AnimationType::Slash).unwrap().current_time = 0.0;
                animator.set_next_animation(AnimationType::Slash);

                controller.time_in_state = 0.0;
                return PlayerState::Attacking
            }

            if rb.linvel().y <= (-(GRAVITY * DECREASED_GRAVITY_SCALAR) + ANIMATION_EPSILON) && controller.time_in_state >= 0.5 {
                animator.set_next_animation(AnimationType::Freefall);
                return PlayerState::Freefalling
            }

            if input.wasd_is_down() {
                sm.play_sound_3d(SoundType::Jump, &trans.position, player_key);
                controller.time_in_state = 0.0;
                return PlayerState::Running;
            }

            return PlayerState::Idle
        },
        // ==================================================================================
        // PLAYER JUMPING
        // ==================================================================================
        PlayerState::Jumping => {
            controller.time_in_state += dt;
            
            if rb.linvel().y <= DECREASED_GRAVITY_SCALAR + ANIMATION_EPSILON {
                controller.time_in_state = 0.0;
                return PlayerState::Freefalling
            }


            //if rb.linvel().y <= ANIMATION_EPSILON && rb.linvel().y >= -ANIMATION_EPSILON {

            //    controller.time_in_state = 0.0;
            //    return PlayerState::Running;
            //}

            return PlayerState::Jumping
        },

        // ==================================================================================
        // PLAYER FREEFALLING
        // ==================================================================================
        PlayerState::Freefalling => {
            controller.time_in_state += dt;

            let grounded = ps.narrow_phase
                .contact_pairs_with(ph.collider)
                .any(|cp| {
                    let is_player_vs_ground =
                    (cp.collider1 == ph.collider && cp.collider2 == ground_ph.collider) ||
                    (cp.collider2 == ph.collider && cp.collider1 == ground_ph.collider);

                    if !is_player_vs_ground { return false; }

                    if cp.has_any_active_contact { return true; } else { return false; }

                });

            if grounded {
                controller.time_in_state = 0.0;
                sm.play_sound_3d(SoundType::Land, &trans.position, player_key);
                return PlayerState::Running;
            }


            if rb.linvel().y <= (-(GRAVITY * DECREASED_GRAVITY_SCALAR) + ANIMATION_EPSILON) && controller.time_in_state >= 0.5 {
                if let Some(_) = animator.animations.get(&AnimationType::Freefall) {
                    animator.set_next_animation(AnimationType::Freefall);
                } else {
                    animator.set_next_animation(AnimationType::Idle);
                }
            }

            return PlayerState::Freefalling
        },
        // ==================================================================================
        // PLAYER RUNNING
        // ==================================================================================
        PlayerState::Running => {
            controller.time_in_state += dt;

            if input.just_pressed(Key::Space) {
                if input.is_down(Key::LeftShift) {
                    rb.apply_impulse(impulse.into(), true);

                    animator.set_next_animation(AnimationType::DashF);

                    return PlayerState::Dashing;
                }

                rb.set_gravity_scale(DECREASED_GRAVITY_SCALAR, true);
                rb.apply_impulse((Vec3::Y * 5.2).into(), true);
                
                if let Some(jump_anim) = animator.animations.get_mut(&AnimationType::Jump) {
                    jump_anim.current_time = 0.0;
                    animator.set_next_animation(AnimationType::Jump);
                } else {
                    animator.set_next_animation(AnimationType::Idle);
                }

                controller.time_in_state = 0.0;
                return PlayerState::Jumping
            }

            if !input.wasd_is_down() {
                controller.time_in_state = 0.0;
                sm.play_sound_3d(SoundType::Jump, &trans.position, player_key);
                return PlayerState::Idle
            }

            if input.mouse_just_pressed(MouseButton::Left) {
                animator.animations.get_mut(&AnimationType::Slash).unwrap().current_time = 0.0;
                animator.set_next_animation(AnimationType::Slash);

                controller.attack_state = AttackState::Attack1;

                controller.time_in_state = 0.0;
                return PlayerState::Attacking
            }

            if rb.linvel().y <= (-(GRAVITY * DECREASED_GRAVITY_SCALAR) + ANIMATION_EPSILON) && controller.time_in_state >= 0.5 {
                animator.set_next_animation(AnimationType::Freefall);
                return PlayerState::Freefalling
            }

            return PlayerState::Running
        },
        // ==================================================================================
        // PLAYER ATTACKING
        // ==================================================================================
        PlayerState::Attacking => {
            // AttackState state machine (kinda a mini state machine)
            match controller.attack_state {
                AttackState::Attack1 => {
                    // lock into attacking until we are fully transitioned into new anim
                    if animator.current_animation != AnimationType::Slash {
                        return PlayerState::Attacking;
                    }

                    let anim = animator.animations.get(&animator.current_animation).unwrap();

                    if anim.current_segment >= 12 && anim.current_segment < 22 {
                        if input.mouse_just_pressed(MouseButton::Left) {
                            animator.animations.get_mut(&AnimationType::Slash2).unwrap().current_time = 0.0;
                            animator.set_next_animation(AnimationType::Slash2);
                            controller.attack_state = AttackState::Attack2;

                            return PlayerState::Attacking;
                        }
                    }

                    if anim.current_segment >= 22 {
                        if input.wasd_is_down() {
                            animator.set_next_animation(AnimationType::Run);
                            controller.attack_state = AttackState::Attack1;

                            controller.time_in_state = 0.0;

                            return PlayerState::Running;
                        }
                    }

                    if anim.current_time >= anim.duration- ANIMATION_EPSILON {
                        animator.set_next_animation(AnimationType::Idle);
                        controller.attack_state = AttackState::Attack1;
                        controller.time_in_state = 0.0;
                        return PlayerState::Idle;
                    }
                },
                AttackState::Attack2 => {
                    if animator.current_animation != AnimationType::Slash2 {
                        return PlayerState::Attacking;
                    }

                    let anim = animator.animations.get(&animator.current_animation).unwrap();

                    if anim.current_segment >= 12
                        && anim.current_segment < 22
                        && input.just_pressed(Key::Space) {

                        controller.attack_state = AttackState::Attack1;

                        let yaw = em.yaws.get(player_key).unwrap();
                        let dir = vec3(yaw.sin(), 1.0, yaw.cos()).normalize();

                        rb.apply_impulse(impulse.into(), true);

                        animator.set_next_animation(AnimationType::DashF);

                        return PlayerState::Dashing;
                    }

                    if anim.current_segment >= 22 {
                        if input.wasd_is_down() {
                            animator.set_next_animation(AnimationType::Run);
                            controller.attack_state = AttackState::Attack1;

                            controller.time_in_state = 0.0;

                            return PlayerState::Running;
                        }
                    }
                    if anim.current_time >= anim.duration - ANIMATION_EPSILON {
                        animator.set_next_animation(AnimationType::Idle);
                        controller.attack_state = AttackState::Attack1;
                        controller.time_in_state = 0.0;

                        return PlayerState::Idle;
                    }

                },
                _ => {},
            }

            return PlayerState::Attacking;
        }
        // ==================================================================================
        // PLAYER DASHING
        // ==================================================================================
        PlayerState::Dashing => {
            if animator.current_animation != AnimationType::DashF {
                animator.set_next_animation(AnimationType::DashF);
                return PlayerState::Dashing;
            }

            let anim = animator.animations.get(&animator.current_animation).unwrap();

            if input.wasd_is_down() && anim.current_segment >= 12 {
                controller.time_in_state = 0.0;
                animator.set_next_animation(AnimationType::Run);
                return PlayerState::Running;
            } 

            if anim.current_time >= anim.duration - ANIMATION_EPSILON {
                controller.time_in_state = 0.0;
                return PlayerState::Idle;
            }

            return PlayerState::Dashing;
        },
        // ==================================================================================
        // PLAYER DYING
        // ==================================================================================
        PlayerState::Dying => {

            return PlayerState::Dying;
        },
        // ==================================================================================
        // PLAYER DEAD
        // ==================================================================================
        PlayerState::Dead {time, target_time} => {
            return PlayerState::Dead {time, target_time}
        },
        PlayerState::Blocking => {
            return PlayerState::Blocking;
        },
    })();

    controller.state = next_state;
}

