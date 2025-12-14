use crate::entity_manager::EntityManager;

pub fn update(em: &mut EntityManager, dt: f32) {
    for entry in em.skellingtons.iter_mut() {
        let animator = em.animators.get_mut(entry.key()).unwrap();
        let skellington = entry.value_mut();

        animator.update(skellington, dt);
    }
}
