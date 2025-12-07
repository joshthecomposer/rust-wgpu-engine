use crate::{camera::Camera, config::world_data::WorldData, entity_manager::EntityManager, lights::Lights, particles::ParticleSystem};

pub struct World {
    pub ecs: EntityManager,
    pub camera: Camera,
    pub lights: Lights,
    pub particles: ParticleSystem,
}

impl World {
    pub fn new() -> Self {
        let ecs = EntityManager::new(10_000);
        let camera = Camera::new();
        let lights = Lights::new(50);
        let particles = ParticleSystem::new("config/emitter_data.toml");

        Self {
            ecs,
            camera,
            lights,
            particles,
        }
    }
}
