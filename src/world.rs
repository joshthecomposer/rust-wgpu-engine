use crate::{camera::Camera, entity_manager::EntityManager, lights::Lights};

pub struct World {
    pub ecs: EntityManager,
    pub camera: Camera,
    pub lights: Lights,
}

impl World {
    pub fn new() -> Self {
        let ecs = EntityManager::new(10_000);
        let camera = Camera::new();
        let lights = Lights::new(50);

        Self {
            ecs,
            camera,
            lights,
        }
    }
}
