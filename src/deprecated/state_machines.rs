use glam::{vec3, Mat4, Vec3};
use glfw::{Key, MouseButton};
use rapier3d::prelude::RigidBodyType;

use crate::{animation::animation::Animator, entity_manager::EntityManager, enums_types::{AnimationType, AttackState, Effect, EmitterName, EntityType, Faction, Knockback, PlayerState, SimState, SimStateController, SoundType, VisualEffect, ANIMATION_EPSILON}, input::InputState, particles::{Emitter, ParticleSystem}, physics::PhysicsState, some_data::{DECREASED_GRAVITY_SCALAR, GRAVITY}, sound::sound_manager::SoundManager, util::data_structure::HashMapGetPairMut};

pub fn update(em: &mut EntityManager, dt: f32, particles: &mut ParticleSystem, input: &InputState, ps: &mut PhysicsState, sm: &mut SoundManager) {
    // COMMON DATA BETWEEN MACHINES
    let player_id = em.factions.iter().find(|e| *e.value() == Faction::Player).unwrap().key();
    player_state_machine(em, dt, input, ps, sm, particles);
        
    // OPTIMIZATION: gather entity IDs once somewhere and use for the entire game loop?
    let enemy_ids = em.factions.iter().filter(|e| *e.value() == Faction::Enemy).map(|e| e.key()).collect::<Vec<usize>>();
    for id in enemy_ids.iter() {
        entity_sim_state_machine(*id, em, dt, particles, ps, input, player_id);
    }
}



fn player_state_machine(em: &mut EntityManager, dt: f32, input: &InputState, ps: &mut PhysicsState, sm: &mut SoundManager,particles: &mut ParticleSystem){ 
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

    let kb = em.knockbacks.get_mut(player_key);

    let next_state = (|| match controller.state {
        // ==================================================================================
        // PLAYER IDLE 
        // ==================================================================================
        PlayerState::Idle => {
            controller.time_in_state += dt;
            animator.set_next_animation(AnimationType::Idle);

            if input.mouse_is_down(MouseButton::Right) {
                controller.time_in_state = 0.0;
                animator.set_next_animation(AnimationType::Block);
                return PlayerState::Blocking;
            }

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
                animator.set_next_animation(AnimationType::Run);
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
controller.time_in_state += dt;

            if controller.time_in_state >= 0.04 {
                controller.time_in_state = 0.0;
                particles.spawn_oneshot_emitter(EmitterName::DesertDust, trans.position);
            }
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
        // ==================================================================================
        // PLAYER BLOCKING
        // ==================================================================================
        PlayerState::Blocking => {
            controller.time_in_state += dt;
            if animator.current_animation != AnimationType::Block {
                return PlayerState::Blocking;
            }

            let anim = animator.animations.get_mut(&animator.current_animation).unwrap();

            if input.mouse_is_down(MouseButton::Right) {
                if let Some(hold_frame) = anim.hold_frame {
                    if anim.current_segment == hold_frame  {
                        anim.do_hold = true;
                        return PlayerState::Blocking;
                    }
                }
            }

            anim.do_hold = false;

            if input.wasd_is_down() && anim.current_segment > 6 {
                controller.time_in_state = 0.0;
                animator.set_next_animation(AnimationType::Run);
                return PlayerState::Running;
            }

            if input.mouse_just_pressed(MouseButton::Left) && anim.current_segment >= 6 {
                controller.time_in_state = 0.0;
                controller.attack_state = AttackState::Attack1;
                animator.set_next_animation(AnimationType::Slash);
                return PlayerState::Attacking;
            }

            if anim.current_time >= anim.duration - ANIMATION_EPSILON {
                controller.time_in_state = 0.0;
                animator.set_next_animation(AnimationType::Idle);
                return PlayerState::Idle;
            }

            return PlayerState::Blocking;
        },
    })();

    if let Some(kb) = kb {
        if !kb.did_particles && next_state != PlayerState::Blocking {
            let model_transform = Mat4::from_scale_rotation_translation(trans.scale, trans.rotation, trans.position);
            let skellington = em.skellingtons.get_mut(player_key).unwrap();

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
                    particles.spawn_oneshot_emitter(EmitterName::DamageBlood, position);
                }
            }
            kb.did_particles = true;
        }
    }

    controller.state = next_state;
}

