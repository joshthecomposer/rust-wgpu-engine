use crate::{
    camera::Camera, entity_manager::EntityManager, lights::Lights, particles::ParticleSystem,
    spawn_system::SpawnManager,
};

pub struct World {
    pub ecs: EntityManager,
    pub camera: Camera,
    pub lights: Lights,
    pub particles: ParticleSystem,
    pub spawn_manager: SpawnManager,
}

impl World {
    pub fn new() -> Self {
        let ecs = EntityManager::new(10_000);
        let camera = Camera::new();
        let lights = Lights::new(50);
        let particles = ParticleSystem::new("config/emitter_data.json");
        let spawn_manager = SpawnManager {
            spawn_every: 5.0,
            amount_per: 1,
            accumulator: 0.0,
        };

        Self {
            ecs,
            camera,
            lights,
            particles,
            spawn_manager,
        }
    }
}
