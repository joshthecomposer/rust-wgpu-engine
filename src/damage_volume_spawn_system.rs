use crate::{entity_manager::EntityManager, enums_types::DamageSource, physics::PhysicsState};

pub fn update(em: &mut EntityManager, ps: &mut PhysicsState) {
    let source_ids = em
        .animators
        .iter()
        .map(|entry| entry.key())
        .collect::<Vec<_>>();

    for source_id in source_ids {
        let Some(animator) = em.animators.get(source_id) else {
            return;
        };

        let anim_type = animator.next_animation;

        let anim_has_damage_volume = em.animation_to_damage_volume.contains_key(&anim_type);

        if anim_has_damage_volume {
            let has_active_damage_volume = em
                .damage_volumes
                .iter()
                .any(|entry| entry.value().was_spawned_by(source_id, anim_type));

            if !has_active_damage_volume {
                em.create_damage_volume(source_id, &anim_type, ps);
            }
        } else {
            let volume_ids = em
                .damage_volumes
                .iter()
                .filter(|entry| entry.value().source.entity_id() == Some(source_id))
                .map(|entry| entry.key())
                .collect::<Vec<_>>();

            for volume_id in volume_ids {
                em.entity_trashcan.push(volume_id);
            }
        }
    }
}
