use glam::{Mat4, Vec3};

use crate::{entity_manager::EntityManager, enums_types::{AnimationType, Faction, SimState, VisualEffect, ANIMATION_EPSILON}, particles::ParticleSystem};

pub fn update(em: &mut EntityManager, dt: f32, particles: &mut ParticleSystem) {
    entity_sim_state_machine(em, dt, particles);
}

fn entity_sim_state_machine(em: &mut EntityManager, dt: f32, particles: &mut ParticleSystem) {
    for fac in em.factions.iter() {
        if *fac.value() == Faction::Enemy {
            let state = em.sim_states.get_mut(fac.key()).unwrap();
            let player_key = em.factions.iter().find(|e| *e.value() == Faction::Player).unwrap().key();
            let player_pos = em.transforms.get(player_key).unwrap().position;
            let entity_pos = em.transforms.get(fac.key()).unwrap().position;
            let animator = em.animators.get_mut(fac.key()).unwrap();
            let destination = em.destinations.get_mut(fac.key()).unwrap();

            let trans = em.transforms.get(fac.key()).unwrap();

            let next_state = (|| match state {
                SimState::Dancing => {
                    *destination = entity_pos;
                    animator.set_next_animation(AnimationType::Dance);
                    SimState::Dancing
                },
                SimState::Waiting => {
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
                    animator.set_next_animation(AnimationType::Run);
                    *destination = player_pos;

                    if entity_pos.distance(player_pos) > 12.0 {
                        return SimState::Waiting
                    } 

                    SimState::Aggro
                },
                SimState::Dying => {
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

                    let new_time = *time + dt;

                    if new_time >= 4.0 {
                        em.v_effects.insert(fac.key(), VisualEffect::Flashing);
                    }

                    if new_time >= *target_time {
                        let model_transform = Mat4::from_scale_rotation_translation(trans.scale, trans.rotation, trans.position);
                        let skellington = em.skellingtons.get_mut(fac.key()).unwrap();

                        let bone_names: Vec<String> = {
                            let anim = animator.animations.get(&animator.current_animation).unwrap();
                            anim.model_animation_join.iter().map(|b| b.name.clone()).collect()
                        };

                        dbg!(&bone_names.len());

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

                    SimState::Dead { time: new_time, target_time: *target_time }
                },
            })();

            *state = next_state;
        }
    }
}
