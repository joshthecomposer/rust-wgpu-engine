use glam::{vec3, Vec3};

use crate::{
    command_buffer::{CommandBuffer, PartCmd, PartKind},
    entity_manager::EntityManager,
    enums_types::{
        DamagePayload, DamageSource, Knockback, LifeState, StatusEffect, StatusEffectHelper,
    },
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

    let animator = match em.animators.get(source_id) {
        Some(a) => a,
        None => return,
    };

    let active = animator
        .get_next_animation()
        .and_then(|anim| anim.hurtbox_activation.as_ref())
        .is_some_and(|ha_list| ha_list.iter().any(|fa| fa.triggered.get()));

    if !active {
        if let Some(hitset) = em.hitsets.get_mut(active_weapon_id) {
            hitset.clear();
        }
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
        Some(f) => f.clone(),
        None => return,
    };

    if !matches!(source_faction.as_str(), "Player" | "Enemy") {
        return;
    }

    let payload = payload_for_source_animation(em, source_id, 1.0);

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

        let Some(victim_faction) = em.factions.get(victim_id).cloned() else {
            continue;
        };

        if !matches!(victim_faction.as_str(), "Player" | "Enemy") {
            continue;
        }

        match (source_faction.as_str(), victim_faction.as_str()) {
            ("Player", "Player") => continue,
            ("Enemy", "Enemy") => continue,
            _ => (),
        }

        let did_insert = match em.hitsets.get_mut(active_weapon_id) {
            Some(hitset) => hitset.insert(other),
            None => return,
        };

        if !did_insert {
            continue;
        }

        let Some(rb_handle) = em.physics_handles.get(victim_id).map(|ph| ph.rigid_body) else {
            continue;
        };

        apply_damage_payload(
            em,
            cmds,
            DamageSource::Entity(source_id),
            victim_id,
            victim_faction.as_str(),
            &payload,
        );

        if let Some(rb) = ps.rigid_body_set.get_mut(rb_handle) {
            let kb = Knockback {
                ttl: 0.35,
                flinch: false,
                did_particles: false,
            };
            let dir = vec3(yaw.sin(), 1.0, yaw.cos()).normalize();

            physics::apply_delta_v(rb, dir, 2.0);
            em.knockbacks.insert(victim_id, kb);
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
        Some(f) => f.clone(),
        None => return,
    };

    if !matches!(source_faction.as_str(), "Player" | "Enemy") {
        return;
    }

    let payload = DamagePayload {
        damage: 3.0,
        status_effects: vec![],
    };

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

        let Some(victim_faction) = em.factions.get(victim_id).cloned() else {
            continue;
        };

        if !matches!(victim_faction.as_str(), "Player" | "Enemy") {
            continue;
        }

        match (source_faction.as_str(), victim_faction.as_str()) {
            ("Player", "Player") => continue,
            ("Enemy", "Enemy") => continue,
            _ => (),
        }

        if em.physics_handles.get(victim_id).is_none() {
            continue;
        }

        apply_damage_payload(
            em,
            cmds,
            DamageSource::Entity(source_id),
            victim_id,
            victim_faction.as_str(),
            &payload,
        );
    }
}

fn resolve_damage_volume_hits(
    dv_id: usize,
    em: &mut EntityManager,
    ps: &mut PhysicsState,
    cmds: &mut CommandBuffer,
    dt: f32,
) {
    let (source, payload) = {
        let dv = em.damage_volumes.get_mut(dv_id).unwrap();

        dv.ticker.tick_accumulator += dt;

        if dv.ticker.tick_accumulator < dv.ticker.tick_ttl {
            return;
        }

        dv.ticker.tick_accumulator -= dv.ticker.tick_ttl;
        (dv.source.clone(), dv.damage_payload.clone())
    };

    let Some(source_id) = source.entity_id() else {
        eprintln!("No entity id on damage source, world versions are not allowed yet");
        return;
    };

    let source_faction = match em.factions.get(source_id) {
        Some(f) => f.clone(),
        None => return,
    };

    if !matches!(source_faction.as_str(), "Player" | "Enemy") {
        return;
    }

    let dv_col_handle = match em.physics_handles.get(dv_id) {
        Some(ph) => ph.collider,
        None => {
            eprintln!("NO COLLIDERER");
            return;
        }
    };

    let Some(hitset) = em.hitsets.get_mut(dv_id) else {
        return;
    };
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

        let Some(victim_faction) = em.factions.get(victim_id).cloned() else {
            continue;
        };

        if !matches!(victim_faction.as_str(), "Player" | "Enemy") {
            continue;
        }

        match (source_faction.as_str(), victim_faction.as_str()) {
            ("Player", "Player") => continue,
            ("Enemy", "Enemy") => continue,
            _ => (),
        }

        let did_insert = match em.hitsets.get_mut(dv_id) {
            Some(hitset) => hitset.insert(other),
            None => return,
        };

        if !did_insert {
            continue;
        }

        if em.physics_handles.get(victim_id).is_none() {
            continue;
        }

        apply_damage_payload(
            em,
            cmds,
            source.clone(),
            victim_id,
            victim_faction.as_str(),
            &payload,
        );
    }
}

fn payload_for_source_animation(
    em: &EntityManager,
    source_id: usize,
    fallback_damage: f32,
) -> DamagePayload {
    let Some(animator) = em.animators.get(source_id) else {
        return basic_payload(fallback_damage);
    };

    let anim_name = animator.next_animation.to_string();

    em.abilities_config
        .abilities
        .iter()
        .find(|ability| ability.animation == anim_name)
        .and_then(|ability| ability.payload.clone())
        .unwrap_or_else(|| basic_payload(fallback_damage))
}

fn basic_payload(damage: f32) -> DamagePayload {
    DamagePayload {
        damage,
        status_effects: vec![],
    }
}

fn apply_damage_payload(
    em: &mut EntityManager,
    cmds: &mut CommandBuffer,
    source: DamageSource,
    victim_id: usize,
    victim_faction: &str,
    payload: &DamagePayload,
) {
    let Some(health_after_damage) = apply_health_damage(em, victim_id, payload.damage) else {
        return;
    };

    for effect in &payload.status_effects {
        insert_status_effect_if_missing(em, victim_id, source.clone(), effect);
    }

    spawn_damage_particles(em, cmds, victim_id);
    mark_victim_damaged(em, victim_id, victim_faction, health_after_damage);
}

fn apply_health_damage(em: &mut EntityManager, victim_id: usize, damage: f32) -> Option<f32> {
    let health = em.healths.get_mut(victim_id)?;
    *health -= damage;
    Some(*health)
}

fn insert_status_effect_if_missing(
    em: &mut EntityManager,
    victim_id: usize,
    source: DamageSource,
    effect: &StatusEffectHelper,
) {
    if em.status_effects.get(victim_id).is_none() {
        em.status_effects.insert(victim_id, Vec::new());
    }

    let effects = em.status_effects.get_mut(victim_id).unwrap();

    if effects.iter().any(|active| active.kind == effect.kind) {
        return;
    }

    effects.push(StatusEffect {
        kind: effect.kind.clone(),
        source,
        remaining: effect.remaining,
        tick_accumulator: 0.0,
        stacks: 1,
        behaviors: effect.behaviors.clone(),
    });
}

fn spawn_damage_particles(em: &EntityManager, cmds: &mut CommandBuffer, victim_id: usize) {
    let Some(t) = em.transforms.get(victim_id) else {
        return;
    };

    let Some(skellington) = em.skellingtons.get(victim_id) else {
        return;
    };

    let entity_world = glam::Mat4::from_scale_rotation_translation(t.scale, t.rotation, t.position);
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
}

fn mark_victim_damaged(
    em: &mut EntityManager,
    victim_id: usize,
    victim_faction: &str,
    health_after_damage: f32,
) {
    match victim_faction {
        "Player" => {
            let Some(target_ctrl) = em.player_controllers.get_mut(victim_id) else {
                return;
            };
            target_ctrl.took_damage = true;
        }
        "Enemy" => {
            let Some(target_ctrl) = em.enemy_controllers.get_mut(victim_id) else {
                return;
            };
            target_ctrl.took_damage = true;
        }
        _ => (),
    }
}
