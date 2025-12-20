use crate::entity_manager::EntityManager;

pub fn update(em: &mut EntityManager) {
    let spawn_area_entry = em
        .entity_types
        .iter()
        .find(|e| e.value() == "MainSpawnableArea");

    if spawn_area_entry.is_none() {
        return;
    }

    let spawn_area_id = spawn_area_entry.unwrap().key();
    let trans = em.collider_transforms.get(spawn_area_id);
}
