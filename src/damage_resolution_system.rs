use glam::{vec3, Vec3};

use crate::{
    command_buffer::{CommandBuffer, PartCmd, PartKind},
    entity_manager::EntityManager,
    enums_types::{Knockback, LifeState},
    physics::{self, PhysicsState},
};

pub fn update(em: &mut EntityManager, _dt: f32, ps: &mut PhysicsState, cmds: &mut CommandBuffer) {
    if let Some(pid) = em.get_player_id() {
        resolve_melee_hits_for_source(pid, em, ps, cmds);
    } else {
        eprintln!("There is no player");
    }

    for eid in em.get_ids_for_faction("Enemy") {
        resolve_melee_hits_for_source(eid, em, ps, cmds);
    }

    let source_ids = em
        .source_ids
        .iter()
        .map(|e| (e.key(), *e.value()))
        .collect::<Vec<(usize, usize)>>();

    for entry in source_ids {
        resolve_projectile_hits_for_source(entry.0, entry.1, em, ps, cmds);
    }

    let dv_ids = em
        .damage_volumes
        .iter()
        .map(|e| e.key())
        .collect::<Vec<usize>>();

    for id in dv_ids {
        resolve_damage_volume_hits(id, em, ps, cmds, _dt);
    }
}

fn resolve_melee_hits_for_source(
    source_id: usize,
    em: &mut EntityManager,
    ps: &mut PhysicsState,
    cmds: &mut CommandBuffer,
) {
    let source_pill_handle = match em.physics_handles.get(source_id) {
        Some(ph) => ph.collider,
        None => {
            eprintln!("NO COLLIDERER");
            return;
        }
    };

    let active_weapon_id = match em.active_items.get(source_id) {
        Some(items) => match items.right_hand {
            Some(id) => id,
            None => return,
        },
        None => return,
    };

    let hitset = match em.hitsets.get_mut(active_weapon_id) {
        Some(h) => h,
        None => return,
    };

    let animator = match em.animators.get(source_id) {
        Some(a) => a,
        None => return,
    };

    let active = animator
        .get_next_animation()
        .and_then(|anim| anim.hurtbox_activation.as_ref())
        .is_some_and(|ha_list| ha_list.iter().any(|fa| fa.triggered.get()));

    if !active {
        hitset.clear();
        return;
    }

    let rh_w_col_handle = match em.physics_handles.get(active_weapon_id) {
        Some(ph) => ph.collider,
        None => return,
    };

    let yaw = match em.yaws.get(source_id) {
        Some(y) => *y,
        None => return,
    };

    let source_faction = match em.factions.get(source_id) {
        Some(f) => f.as_str(),
        None => return,
    };

    if !matches!(source_faction, "Player" | "Enemy") {
        return;
    }

    for (c1, c2, i) in ps.narrow_phase.intersection_pairs_with(rh_w_col_handle) {
        if !i {
            continue;
        }

        if c1 == source_pill_handle || c2 == source_pill_handle {
            continue;
        }

        let other = if c1 == rh_w_col_handle { c2 } else { c1 };

        let Some(&victim_id) = em.collider_to_entity.get(&other) else {
            eprintln!(
                "collider {:?} has no entity; likely stale pair or missing insert",
                other
            );
            continue;
        };

        if victim_id == source_id {
            continue;
        }

        let Some(victim_faction) = em.factions.get(victim_id).map(|f| f.as_str()) else {
            continue;
        };

        if !matches!(victim_faction, "Player" | "Enemy") {
            continue;
        }

        match (source_faction, victim_faction) {
            ("Player", "Player") => continue,
            ("Enemy", "Enemy") => continue,
            _ => (),
        }

        if !hitset.insert(other) {
            continue;
        }

        if let Some(ph) = em.physics_handles.get(victim_id) {
            if let Some(rb) = ps.rigid_body_set.get_mut(ph.rigid_body) {
                let health = em.healths.get_mut(victim_id).unwrap();

                *health -= 1.0;

                let kb = Knockback {
                    ttl: 0.35,
                    flinch: false,
                    did_particles: false,
                };

                let t = em.transforms.get(victim_id).unwrap();

                let entity_world =
                    glam::Mat4::from_scale_rotation_translation(t.scale, t.rotation, t.position);

                let skellington = em.skellingtons.get(victim_id).unwrap();

                let mut stack = Vec::new();

                for bone in &skellington.children {
                    stack.push(bone);
                }

                while let Some(bone) = stack.pop() {
                    let bone_world = entity_world * bone.global_transform;
                    let pos = bone_world.w_axis.truncate();

                    cmds.particles.push(PartCmd {
                        name: "DamageBlood".to_string(),
                        kind: PartKind::WorldOrigin(pos),
                        direction: Vec3::Y,
                    });

                    for child in &bone.children {
                        stack.push(child);
                    }
                }

                match victim_faction {
                    "Player" => {
                        let target_ctrl = em.player_controllers.get_mut(victim_id).unwrap();
                        target_ctrl.took_damage = true;
                        if *health <= 0.0 {
                            if !matches!(target_ctrl.life_state, LifeState::Dying | LifeState::Dead)
                            {
                                target_ctrl.life_state = LifeState::Dying
                            }
                        }
                    }
                    "Enemy" => {
                        let target_ctrl = em.enemy_controllers.get_mut(victim_id).unwrap();
                        target_ctrl.took_damage = true;
                        if *health <= 0.0 {
                            if !matches!(target_ctrl.life_state, LifeState::Dying | LifeState::Dead)
                            {
                                target_ctrl.life_state = LifeState::Dying
                            }
                        }
                    }
                    _ => (),
                }

                let dir = vec3(yaw.sin(), 1.0, yaw.cos()).normalize();

                physics::apply_delta_v(rb, dir, 2.0);
                em.knockbacks.insert(victim_id, kb);
            }
        }
    }
}

fn resolve_projectile_hits_for_source(
    proj_id: usize,
    source_id: usize,
    em: &mut EntityManager,
    ps: &mut PhysicsState,
    cmds: &mut CommandBuffer,
) {
    let source_faction = match em.factions.get(source_id) {
        Some(f) => f.as_str(),
        None => return,
    };

    if !matches!(source_faction, "Player" | "Enemy") {
        return;
    }

    let proj_col_handle = match em.physics_handles.get(proj_id) {
        Some(ph) => ph.collider,
        None => {
            eprintln!("NO COLLIDERER");
            return;
        }
    };

    for (c1, c2, i) in ps.narrow_phase.intersection_pairs_with(proj_col_handle) {
        if !i {
            continue;
        }

        let other = if c1 == proj_col_handle {
            c2
        } else if c2 == proj_col_handle {
            c1
        } else {
            continue;
        };

        let Some(&victim_id) = em.collider_to_entity.get(&other) else {
            continue;
        };

        if victim_id == source_id {
            continue;
        }

        let Some(victim_faction) = em.factions.get(victim_id).map(|f| f.as_str()) else {
            continue;
        };

        if !matches!(victim_faction, "Player" | "Enemy") {
            continue;
        }

        match (source_faction, victim_faction) {
            ("Player", "Player") => continue,
            ("Enemy", "Enemy") => continue,
            _ => (),
        }

        if let Some(ph) = em.physics_handles.get(victim_id) {
            let health = em.healths.get_mut(victim_id).unwrap();

            *health -= 3.0;

            let t = em.transforms.get(victim_id).unwrap();

            let entity_world =
                glam::Mat4::from_scale_rotation_translation(t.scale, t.rotation, t.position);

            let skellington = em.skellingtons.get(victim_id).unwrap();

            let mut stack = Vec::new();

            for bone in &skellington.children {
                stack.push(bone);
            }

            while let Some(bone) = stack.pop() {
                let bone_world = entity_world * bone.global_transform;
                let pos = bone_world.w_axis.truncate();

                cmds.particles.push(PartCmd {
                    name: "DamageBlood".to_string(),
                    kind: PartKind::WorldOrigin(pos),
                    direction: Vec3::Y,
                });

                for child in &bone.children {
                    stack.push(child);
                }
            }

            match victim_faction {
                "Player" => {
                    let target_ctrl = em.player_controllers.get_mut(victim_id).unwrap();
                    target_ctrl.took_damage = true;
                    if *health <= 0.0 {
                        if !matches!(target_ctrl.life_state, LifeState::Dying | LifeState::Dead) {
                            target_ctrl.life_state = LifeState::Dying
                        }
                    }
                }
                "Enemy" => {
                    let target_ctrl = em.enemy_controllers.get_mut(victim_id).unwrap();
                    target_ctrl.took_damage = true;
                    if *health <= 0.0 {
                        if !matches!(target_ctrl.life_state, LifeState::Dying | LifeState::Dead) {
                            target_ctrl.life_state = LifeState::Dying
                        }
                    }
                }
                _ => (),
            }
        }
    }
}

fn resolve_damage_volume_hits(
    dv_id: usize,
    em: &mut EntityManager,
    ps: &mut PhysicsState,
    cmds: &mut CommandBuffer,
    dt: f32,
) {
    let dv = em.damage_volumes.get_mut(dv_id).unwrap();

    let source_id = dv.source_id.unwrap();

    let source_faction = match em.factions.get(source_id) {
        Some(f) => f.as_str(),
        None => return,
    };

    if !matches!(source_faction, "Player" | "Enemy") {
        return;
    }

    let dv_col_handle = match em.physics_handles.get(dv_id) {
        Some(ph) => ph.collider,
        None => {
            eprintln!("NO COLLIDERER");
            return;
        }
    };

    let hitset = match em.hitsets.get_mut(dv_id) {
        Some(h) => h,
        None => return,
    };

    dv.ticker.tick_accumulator += dt;

    if dv.ticker.tick_accumulator < dv.ticker.tick_ttl {
        return;
    }

    dv.ticker.tick_accumulator -= dv.ticker.tick_ttl;
    hitset.clear();

    for (c1, c2, i) in ps.narrow_phase.intersection_pairs_with(dv_col_handle) {
        if !i {
            continue;
        }

        let other = if c1 == dv_col_handle {
            c2
        } else if c2 == dv_col_handle {
            c1
        } else {
            continue;
        };

        let Some(&victim_id) = em.collider_to_entity.get(&other) else {
            continue;
        };

        if victim_id == source_id {
            continue;
        }

        let Some(victim_faction) = em.factions.get(victim_id).map(|f| f.as_str()) else {
            continue;
        };

        if !matches!(victim_faction, "Player" | "Enemy") {
            continue;
        }

        match (source_faction, victim_faction) {
            ("Player", "Player") => continue,
            ("Enemy", "Enemy") => continue,
            _ => (),
        }

        if !hitset.insert(other) {
            continue;
        }

        if let Some(ph) = em.physics_handles.get(victim_id) {
            let health = em.healths.get_mut(victim_id).unwrap();

            *health -= dv.damage_scalar;

            let t = em.transforms.get(victim_id).unwrap();

            let entity_world =
                glam::Mat4::from_scale_rotation_translation(t.scale, t.rotation, t.position);

            let skellington = em.skellingtons.get(victim_id).unwrap();

            let mut stack = Vec::new();

            for bone in &skellington.children {
                stack.push(bone);
            }

            while let Some(bone) = stack.pop() {
                let bone_world = entity_world * bone.global_transform;
                let pos = bone_world.w_axis.truncate();

                cmds.particles.push(PartCmd {
                    name: "DamageBlood".to_string(),
                    kind: PartKind::WorldOrigin(pos),
                    direction: Vec3::Y,
                });

                for child in &bone.children {
                    stack.push(child);
                }
            }

            match victim_faction {
                "Player" => {
                    let target_ctrl = em.player_controllers.get_mut(victim_id).unwrap();
                    target_ctrl.took_damage = true;
                    if *health <= 0.0 {
                        if !matches!(target_ctrl.life_state, LifeState::Dying | LifeState::Dead) {
                            target_ctrl.life_state = LifeState::Dying
                        }
                    }
                }
                "Enemy" => {
                    let target_ctrl = em.enemy_controllers.get_mut(victim_id).unwrap();
                    target_ctrl.took_damage = true;
                    if *health <= 0.0 {
                        if !matches!(target_ctrl.life_state, LifeState::Dying | LifeState::Dead) {
                            target_ctrl.life_state = LifeState::Dying
                        }
                    }
                }
                _ => (),
            }
        }
    }
}
