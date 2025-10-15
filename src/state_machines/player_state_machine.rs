use nalgebra::{point, vector};
use rapier3d::prelude::{ColliderSet, ContactPair, InteractionGroups, QueryFilter, QueryPipeline, Ray, RigidBody, RigidBodySet};

use crate::{animation::{animation::Animator, animation_system}, camera::Camera, entity_manager::EntityManager, enums_types::{AnimationType, AttackState, CameraState, EmitterName, EntityType, Faction, PlayerController, PlayerState, SoundType, ANIMATION_EPSILON}, input::InputState, particles::ParticleSystem, physics::{self, PhysicsState}, some_data::{DECREASED_GRAVITY_SCALAR, GRAVITY}, sound::sound_manager::SoundManager, util::data_structure::HashMapGetPairMut};

pub fn player_state_machine(
    em: &mut EntityManager, 
    dt: f32, 
    input: &InputState, 
    ps: &mut PhysicsState, 
    sm: &mut SoundManager,
    particles: &mut ParticleSystem,
    camera: &Camera
) {
    // ==================================================================================
    // BLACKBOARD DATA
    // ==================================================================================
    let player_id   = em.factions.iter().find(|f| *f.value() == Faction::Player).unwrap().key();
    let controller  = em.player_controllers.get_mut(player_id).unwrap();
    let player_pos  = em.transforms.get(player_id).unwrap().position;
    let animator    = em.animators.get_mut(player_id).unwrap();
    let health      = em.healths.get(player_id).unwrap();
    let ph          = em.physics_handles.get(player_id).unwrap();

    // CHECK GROUNDED
    let grounded = is_grounded_ray(
        ps.query_pipeline.as_ref().unwrap(),
        &ps.collider_set,
        &ps.rigid_body_set,
        player_pos,
        0.05,
        0,
    );

    let rb          = ps.rigid_body_set.get_mut(ph.rigid_body).unwrap();
    let yaw         = em.yaws.get(player_id).unwrap();

    let jump_height = em.jump_heights.get(player_id).unwrap();

   let dir = glam::vec3(yaw.sin(), 1.0, yaw.cos()).normalize();
   let m = rb.mass();
   let impulse = glam::vec3(dir.x * (15.0 * m), 0.0, dir.z * (15.0 * m));

    let camera_is_detached = camera.move_state == CameraState::Free;
    
    //player_non_combat_transition(controller, PlayerState::Running, animator, false, rb);
    //return;
    
    // ==================================================================================
    // GUARDS
    // ==================================================================================
    if !grounded && controller.state != PlayerState::Freefalling {
        if rb.linvel().y <= DECREASED_GRAVITY_SCALAR + ANIMATION_EPSILON {
            rb.set_gravity_scale(3.0, true);
        }

        player_non_combat_transition(controller, PlayerState::Freefalling, animator, true, rb);
    }

    if *health <= 0.0 {
        match controller.state {
            PlayerState::Dying | PlayerState::Dead => (),
            _ => return player_non_combat_transition(controller, PlayerState::Dying, animator, false, rb),
        }
    }

    if camera_is_detached {
        if controller.state != PlayerState::Idle {
            player_non_combat_transition(controller, PlayerState::Idle, animator, false, rb);
        }

        return
    }

    // ==================================================================================
    // STATE_MACHINE
    // ==================================================================================
    'ns: {
        match controller.state {
            PlayerState::Init => {
                player_non_combat_transition(controller, PlayerState::Idle, animator, true, rb);
                break 'ns
            },
            PlayerState::Idle => {
                controller.time_in_state += dt;

                if input.wasd_is_down() {
                    player_non_combat_transition(controller, PlayerState::Running, animator, false, rb);
                    sm.play_sound_3d(SoundType::Jump, &player_pos, player_id);
                    break 'ns
                }

                if input.space_just_pressed() && input.shift_is_down() {
                    rb.apply_impulse(impulse.into(), true);
                    player_non_combat_transition(controller, PlayerState::Dashing, animator, true, rb);
                    break 'ns
                }

                if input.space_just_pressed() {
                    rb.apply_impulse(jump_height.precalculated.unwrap(), true);
                    player_non_combat_transition(controller, PlayerState::Jumping, animator, false, rb);
                    break 'ns
                }

                if input.left_mouse_just_pressed() {
                    player_non_combat_transition(controller, PlayerState::Combat, animator, false, rb);
                    break 'ns
                }

                if input.right_mouse_is_down() {
                    player_non_combat_transition(controller, PlayerState::Block, animator, true, rb);
                    break 'ns
                }
            },
            PlayerState::Running     => {
                controller.time_in_state += dt;

                if input.space_just_pressed() && input.shift_is_down() {
                    rb.apply_impulse(impulse.into(), true);
                    player_non_combat_transition(controller, PlayerState::Dashing, animator, true, rb);
                    break 'ns
                }

                if input.space_just_pressed() {
                    rb.apply_impulse(jump_height.precalculated.unwrap(), true);
                    player_non_combat_transition(controller, PlayerState::Jumping, animator, true, rb);
                    break 'ns
                }

                if input.left_mouse_just_pressed() {
                    player_non_combat_transition(controller, PlayerState::Combat, animator, false, rb);
                    break 'ns
                }

                if input.right_mouse_is_down() {
                    player_non_combat_transition(controller, PlayerState::Block, animator, false, rb);
                    break 'ns
                }

                if !input.wasd_is_down() {
                    player_non_combat_transition(controller, PlayerState::Idle, animator, false, rb);
                    sm.play_sound_3d(SoundType::Jump, &player_pos, player_id);
                    break 'ns
                }
            },
            PlayerState::Jumping => {
                controller.time_in_state += dt;

                if grounded && controller.time_in_state >= 0.2 { 
                    player_non_combat_transition(controller, PlayerState::Running, animator, false, rb);
                    sm.play_sound_3d(SoundType::Jump, &player_pos, player_id);
                    break 'ns
                }
            },
            PlayerState::Dashing     => {
                controller.time_in_state += dt;

                if controller.time_in_state >= 0.05 {
                    controller.time_in_state = 0.0;
                    particles.spawn_oneshot_emitter(EmitterName::DesertSlide, player_pos);
                }

                let dash_anim = animator.animations.get(&AnimationType::DashF).unwrap();

                if input.wasd_is_down() && dash_anim.current_segment.get() >= 12 {
                    player_non_combat_transition(controller, PlayerState::Running, animator, false, rb);
                    break 'ns
                }

                if dash_anim.current_time >= dash_anim.duration - ANIMATION_EPSILON {
                    player_non_combat_transition(controller, PlayerState::Idle, animator, false, rb);
                    break 'ns
                }
            },
            PlayerState::Freefalling => {
                controller.time_in_state += dt;
                
                if grounded { 
                    rb.set_gravity_scale(1.0, true);
                    player_non_combat_transition(controller, PlayerState::Running, animator, false, rb);
                    particles.spawn_oneshot_emitter(EmitterName::DesertLand, player_pos);
                    sm.play_sound_3d(SoundType::Land, &player_pos, player_id);
                    break 'ns
                }

                if controller.time_in_state >= 2.0 {
                    animator.set_next_animation(AnimationType::Freefall);
                    break 'ns
                }
            },
            PlayerState::Combat => {
                controller.time_in_state += dt;

                player_combat_state_machine(controller, animator, input, rb);
            },
            PlayerState::Dying       => {},
            PlayerState::Dead        => {},
            PlayerState::Block        => {
                controller.time_in_state += dt;

                let block_anim = animator.animations.get_mut(&AnimationType::Block).unwrap();

                if input.mouse_is_down(glfw::MouseButton::Right) {
                    if let Some(hold_frame) = block_anim.hold_frame {
                        if block_anim.current_segment.get() == hold_frame  {
                            block_anim.do_hold = true;
                            break 'ns
                        }
                    } 

                    break 'ns
                }

                block_anim.do_hold = false;

                if input.wasd_is_down() && block_anim.current_segment.get() > 6 {
                    player_non_combat_transition(controller, PlayerState::Running, animator, false, rb);
                    break 'ns
                }

                if input.left_mouse_just_pressed() && block_anim.current_segment.get() >= 6 {
                    player_non_combat_transition(controller, PlayerState::Combat, animator, false, rb);
                    break 'ns
                }

                if block_anim.current_time >= block_anim.duration - ANIMATION_EPSILON {
                    player_non_combat_transition(controller, PlayerState::Idle, animator, false, rb);
                    break 'ns
                }
            },
        }
    }
}

fn player_combat_state_machine(
    c: &mut PlayerController, 
    a: &mut Animator,
    input: &InputState,
    rb: &mut RigidBody,
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
                if a1.current_segment.get() >= 12 && input.right_mouse_is_down() {
                    player_non_combat_transition(c, PlayerState::Block, a, false, rb);
                    break 'ns
                }

                if a1.current_segment.get() >= 12 && input.left_mouse_just_pressed() {
                    player_combat_transition(c, AttackState::Attack2, a, false);
                    break 'ns
                }

                if a1.current_time >= a1.duration - ANIMATION_EPSILON {
                    player_non_combat_transition(c, PlayerState::Init, a, false, rb);
                    break 'ns
                }
            },
            AttackState::Attack2 => {
                if a2.current_segment.get() >= 12 && input.right_mouse_is_down() {
                    player_non_combat_transition(c, PlayerState::Block, a, false, rb);
                    break 'ns
                }

                if a2.current_time >= a2.duration - ANIMATION_EPSILON {
                    player_non_combat_transition(c, PlayerState::Init, a, false, rb);
                    break 'ns
                }
            },
            _ => unreachable!()
        }
    }
}

fn player_combat_transition(
    c: &mut PlayerController, 
    next_state: AttackState, 
    a: &mut Animator,
    reset_anim: bool,
){
    let anim = match next_state {
        AttackState::Attack1 => AnimationType::Slash,
        AttackState::Attack2 => AnimationType::Slash2,
        AttackState::Attack3 => unreachable!("somehow switched AttackState to Attack3"),
    };

    c.attack_state = next_state;
    c.time_in_state = 0.0;
    c.state = PlayerState::Combat;

    if reset_anim {
        a.set_current_animation(anim.clone());
        a.set_next_animation(anim.clone());
        a.animations.get_mut(&anim.clone()).unwrap().current_time = 0.0;
    } else {
        a.set_next_animation(anim);
    }
}

fn player_non_combat_transition(
    c: &mut PlayerController,
    next_state: PlayerState, 
    a: &mut Animator,
    reset_anim: bool,
    rb: &mut RigidBody,
) {
    let anim = match next_state {
        PlayerState::Init        => AnimationType::Idle,
        PlayerState::Idle        => AnimationType::Idle,
        PlayerState::Dying       => AnimationType::Idle,
        PlayerState::Dead        => AnimationType::Idle,
        // PlayerState from non-combat to combat
        PlayerState::Combat      => AnimationType::Slash,
        PlayerState::Running     => AnimationType::Run,
        PlayerState::Jumping     => { AnimationType::Jump },
        PlayerState::Dashing     => AnimationType::DashF,
        PlayerState::Freefalling => {
            //rb.set_gravity_scale(3.0, true);
            
            a.next_animation.clone()
        },
        PlayerState::Block       => AnimationType::Block,
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

fn is_grounded_ray(
    query: &QueryPipeline,
    colliders: &ColliderSet,
    bodies: &RigidBodySet,
    origin: glam::Vec3,
    max_dist: f32,
    terrain_mask: u32, // pass 0 to not use this
) -> bool {
    let ray = Ray::new(point![origin.x, origin.y, origin.z], vector![0.0, -1.0, 0.0]);

    let filter = QueryFilter::default().groups(InteractionGroups::new(u32::MAX.into(), u32::MAX.into())).into();
    if let Some((handle, toi)) = query.cast_ray(&bodies, &colliders, &ray, max_dist, true, filter) {
        // can add a slope limit here
        return true;
    }
    false
}
