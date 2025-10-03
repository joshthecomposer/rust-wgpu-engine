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

fn entity_sim_state_machine(
    entity_id: usize,
    em: &mut EntityManager,
    dt: f32,
    particles: &mut ParticleSystem,
    ps: &mut PhysicsState,
    input: &InputState,
    player_id: usize,
) {
    // ==================================================================================
    // BLACKBOARD DATA
    // ==================================================================================
    let controller  = em.simstate_controllers.get_mut(entity_id).unwrap();
    let player_pos  = em.transforms.get(player_id).unwrap().position;
    let entity_pos  = em.transforms.get(entity_id).unwrap().position;
    let animator    = em.animators.get_mut(entity_id).unwrap();
    let destination = em.destinations.get_mut(entity_id).unwrap();
    let health      = em.healths.get(entity_id).unwrap();
    let ph          = em.physics_handles.get(entity_id).unwrap();
    let rb          = ps.rigid_body_set.get_mut(ph.rigid_body).unwrap();
    let yaw         = em.yaws.get(entity_id).unwrap();
    let entity_type = em.entity_types.get(entity_id).unwrap();
    let aggro_range = em.aggro_ranges.get(entity_id).unwrap();
    let transform   = em.transforms.get(entity_id).unwrap();

    let kb = em.knockbacks.get_mut(entity_id);

    let entity_cyl = ps.collider_set.get(ph.collider).unwrap();

    let rh_weapon_id = em.active_items.get(entity_id).and_then(|aw| aw.right_hand);
    let can_attack = animator.animations.get(&AnimationType::Slash).is_some() && rh_weapon_id.is_some();
    
    let weapon_length = if can_attack && rh_weapon_id.is_some() {
        em.parents
        .iter()
        .find(|p| p.value().parent_id == rh_weapon_id.unwrap() && em.cuboids.get(p.key()).is_some())
        .and_then(|entry| em.cuboids.get(entry.key()).map(|hb| hb.h))
        .unwrap()
    } else {
        0.0
    };

    let attack_length = weapon_length + entity_cyl.shape().as_capsule().unwrap().radius;
    let within_weapon_length = entity_pos.distance(player_pos) <= attack_length;
    let within_aggro_range = entity_pos.distance(player_pos)   <= *aggro_range;
    let fov_threshold = 0.5;

    let can_see_player = {
        let to_player = (player_pos - entity_pos).with_y(0.0).normalize();
        let forward = (transform.rotation * Vec3::Z).with_y(0.0).normalize();
        let alignment = forward.dot(to_player);

        alignment >= fov_threshold && within_aggro_range
    };

    let anim = animator.get_current_animation().unwrap();
    let anim_type = &animator.current_animation;

    // ==================================================================================
    // STATE_MACHINE
    // ==================================================================================
    // Early return to create dying state:
    if *health <= 0.0 {
        match controller.state {
            SimState::Dying | SimState::Dead => (),
            _ => return entity_non_combat_transition(controller, SimState::Dying, animator, false),
        }
    }

    'ns: {
        match controller.state {
            SimState::Init => {
                if *entity_type == EntityType::MooseMan {
                    entity_non_combat_transition(controller, SimState::Dancing, animator, false);
                    break 'ns;
                }

                entity_non_combat_transition(controller, SimState::Waiting, animator, true);
            },
            SimState::Waiting => {
                controller.time_in_state += dt;
                if let Some(kb) = kb {
                    if kb.ttl > 0.0 && kb.flinch {
                        kb.flinch = false;
                        entity_non_combat_transition(controller, SimState::Flinching, animator, false);
                        reset_combat(controller, animator);
                        break 'ns;
                    }
                }

                if can_see_player {
                    entity_non_combat_transition(controller, SimState::Aggro, animator, false);
                    break 'ns
                }

                *destination = entity_pos;
            },
            SimState::Aggro => {
                controller.time_in_state += dt;

                if let Some(kb) = kb {
                    if kb.ttl > 0.0 && kb.flinch  {
                        kb.flinch = false;
                        entity_non_combat_transition(controller, SimState::Flinching, animator, false);
                        reset_combat(controller, animator);
                        break 'ns;
                    }
                }

                if !within_aggro_range {
                    entity_non_combat_transition(controller, SimState::Waiting, animator, false);
                    break 'ns;
                }

                if within_weapon_length {
                    entity_non_combat_transition(controller, SimState::Combat, animator, false);
                }

                *destination = player_pos;
            },
            SimState::Combat => {
                controller.time_in_state += dt;

                if let Some(kb) = kb {
                    if kb.ttl > 0.0 && kb.flinch  {
                        kb.flinch = false;
                        entity_non_combat_transition(controller, SimState::Flinching, animator, false);
                        reset_combat(controller, animator);
                        break 'ns;
                    }
                }

                *destination = player_pos;
                entity_combat_state_machine(controller, animator, within_weapon_length);
            },
            SimState::Flinching => {
                controller.time_in_state += dt;

                if controller.time_in_state >= anim.duration - ANIMATION_EPSILON {
                    entity_non_combat_transition(controller, SimState::Aggro, animator, false);
                }

                *destination = entity_pos;
            },
            SimState::Dying => {
                controller.time_in_state += dt;
                rb.set_enabled_rotations(true, true, true, true);

                if let Some(rh_weapon_id) = rh_weapon_id {
                    em.parents.remove(rh_weapon_id);
                    em.active_items.remove(entity_id);
                    em.inventories.remove(entity_id);
                }

                if controller.time_in_state >= 3.0 {
                    entity_non_combat_transition(controller, SimState::Dead, animator, false);
                }
            },
            SimState::Dead => {

                let model_transform = Mat4::from_scale_rotation_translation(transform.scale, transform.rotation, transform.position);
                let skellington = em.skellingtons.get_mut(entity_id).unwrap();

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
                        particles.spawn_oneshot_emitter(EmitterName::BodyPoof, position);
                    }
                }

                // if let Some(rh_weapon_id) = rh_weapon_id {
                //     em.parents.remove(rh_weapon_id);
                // }
                
                // remove the hitbox parent
                if let Some(hitbox_parent) = em.parents.iter().find(|p| p.value().parent_id == entity_id) {
                    em.entity_trashcan.push(hitbox_parent.key());
                }
                em.entity_trashcan.push(entity_id);
            },
            SimState::Dancing => {
                // If put in this state you're stuck there.
            },
            SimState::Blocking => {
                unreachable!("An entity shouldn't get to the blocking state right now.");
            },
        }
    }
}

fn entity_combat_state_machine(
    c: &mut SimStateController,
    a: &mut Animator,
    in_range: bool,
) {

    let (a1, a2) = a.animations.get_pair_mut(&AnimationType::Slash, &AnimationType::Slash2).unwrap();
    let current = &a.current_animation;
    
    // reset anims
    match current {
        AnimationType::Slash => a2.current_time = 0.0,
        AnimationType::Slash2 => a1.current_time = 0.0,
        _ => (),
    }

    'ns: {
        match c.attack_state {
            AttackState::Attack1 => {
                if a1.current_time >= a1.duration - ANIMATION_EPSILON {
                    if in_range {
                        entity_combat_transition(c, AttackState::Attack2, a, false);
                        break 'ns
                    } else {
                        entity_non_combat_transition(c, SimState::Aggro, a, false);
                        break 'ns
                    }
                }

                if a1.current_segment >= 16 {
                    if in_range {
                        entity_combat_transition(c, AttackState::Attack2, a, false);
                        break 'ns
                    }
                }
            },
            AttackState::Attack2 => {
                if a2.current_time >= a2.duration - ANIMATION_EPSILON {
                    if in_range {
                        entity_combat_transition(c, AttackState::Attack1, a, false);
                    } else {
                        entity_non_combat_transition(c, SimState::Aggro, a, false);
                        break 'ns
                    }
                }
            },
            AttackState::Attack3 => {
            },
        }
    }
}

fn entity_combat_transition(
    c: &mut SimStateController, 
    next_state: AttackState, 
    a: &mut Animator,
    reset_anim: bool,
){
    let anim = match next_state {
        AttackState::Attack1 => AnimationType::Slash,
        AttackState::Attack2 => AnimationType::Slash2,
        AttackState::Attack3 => unreachable!("somehow switched AttackState to Attack3")
    };

    c.attack_state = next_state;

    c.time_in_state = 0.0;

    if reset_anim {
        a.set_current_animation(anim.clone());
        a.set_next_animation(anim.clone());
        a.animations.get_mut(&anim.clone()).unwrap().current_time = 0.0;
    } else {
        a.set_next_animation(anim);
    }
}

fn reset_combat (
    c: &mut SimStateController, 
    a: &mut Animator,
) {
    let (a1, a2) = a.animations.get_pair_mut(&AnimationType::Slash, &AnimationType::Slash2).unwrap();

    c.attack_state = AttackState::Attack1;
    a1.current_time = 0.0;
    a2.current_time = 0.0;
}

fn entity_non_combat_transition(
    c: &mut SimStateController, 
    next_state: SimState, 
    a: &mut Animator,
    reset_anim: bool,
){
    let anim = match next_state {
            SimState::Init => AnimationType::Idle,
            SimState::Waiting => AnimationType::Idle,
            SimState::Aggro => AnimationType::Run,
            SimState::Dying => AnimationType::Idle,
            SimState::Dead => AnimationType::Idle,
            // going from non-combat to combat
            SimState::Combat => AnimationType::Slash,
            SimState::Flinching => AnimationType::Flinch,
            SimState::Dancing => AnimationType::Dance,
            SimState::Blocking => AnimationType::Block,
    };

    c.state = next_state;
    c.time_in_state = 0.0;
    c.attack_state = AttackState::Attack1;

    if reset_anim {
        // a.set_current_animation(anim.clone());
        a.set_next_animation(anim.clone());
        a.animations.get_mut(&anim.clone()).unwrap().current_time = 0.0;
    } else {
        a.set_next_animation(anim);
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

