use glam::{vec3, Mat4, Vec3};

use crate::{animation::animation::Animator, entity_manager::EntityManager, enums_types::{AnimationType, AttackState, EmitterName, EntityType, Faction, PlayerState, SimState, SimStateController, SoundType, ANIMATION_EPSILON}, input::InputState, particles::ParticleSystem, physics::PhysicsState, some_data::{DECREASED_GRAVITY_SCALAR, GRAVITY}, sound::sound_manager::SoundManager, util::data_structure::HashMapGetPairMut};

pub fn enemy_sim_state_machine(
    entity_id: usize,
    em: &mut EntityManager,
    dt: f32,
    particles: &mut ParticleSystem,
    ps: &mut PhysicsState,
    input: &InputState,
) {
    // ==================================================================================
    // BLACKBOARD DATA
    // ==================================================================================
    let controller  = em.simstate_controllers.get_mut(entity_id).unwrap();
    let animator    = em.animators.get_mut(entity_id).unwrap();
    let entity_pos  = em.transforms.get(entity_id).unwrap().position;
    let destination = em.destinations.get_mut(entity_id).unwrap();
    let entity_type = em.entity_types.get(entity_id).unwrap();
        
    let player_id = match em.factions.iter().find(|f| *f.value() == "Player") {
        Some(e) => Some(e.key()),
        None => None,
    };

    if player_id.is_none() {
        if entity_type == "MooseMan" {
            return;
        };
        entity_non_combat_transition(controller, SimState::Waiting, animator, true);
        *destination = entity_pos;
        return;
    };


    let health      = em.healths.get(entity_id).unwrap();
    let ph          = em.physics_handles.get(entity_id).unwrap();
    let rb          = ps.rigid_body_set.get_mut(ph.rigid_body).unwrap();
    let yaw         = em.yaws.get(entity_id).unwrap();
    let aggro_range = em.aggro_ranges.get(entity_id).unwrap();
    let transform   = em.transforms.get(entity_id).unwrap();
    let player_pos  = em.transforms.get(player_id.unwrap()).unwrap().position;

    let kb = em.knockbacks.get_mut(entity_id);

    let entity_cyl = ps.collider_set.get(ph.collider).unwrap();
    
    let active_weapon_id = match em.active_items.get(entity_id) {
        Some(id) => Some(id.right_hand.unwrap()),
        None => None,
    };

    let can_attack = animator.animations.get(&AnimationType::Slash).is_some() && active_weapon_id.is_some();

    let weapon_length = if let Some(awid) = active_weapon_id {
        *em.model_heights.get(awid).unwrap()
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
    if *health <= 0.0 && entity_type != "MooseMan" {
        match controller.state {
            SimState::Dying | SimState::Dead => (),
            _ => return entity_non_combat_transition(controller, SimState::Dying, animator, false),
        }
    }

    'ns: {
        match controller.state {
            SimState::Init => {
                if entity_type == "MooseMan" {
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

                if let Some(active_weapon_id) = active_weapon_id {
                    em.owners.remove(active_weapon_id);
                    em.is_equipped.remove(active_weapon_id);
                    em.active_items.remove(entity_id);
                    em.cleanup_timer.insert(active_weapon_id, 0.0);

                    if let Some(inv) = em.inventories.get_mut(entity_id) {
                        inv.retain(|v| *v != active_weapon_id);
                    }
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
                        particles.spawn_oneshot_emitter("BodyPoof", position, None);
                    }
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

                if a1.current_segment.get() >= 16 {
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
        AttackState::Attack3 => unreachable!("somehow switched AttackState to Attack3"),
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
    if let Some((a1, a2)) = a.animations.get_pair_mut(&AnimationType::Slash, &AnimationType::Slash2) {
        c.attack_state = AttackState::Attack1;
        a1.current_time = 0.0;
        a2.current_time = 0.0;
    }
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
