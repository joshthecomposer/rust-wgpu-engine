use crate::{
    camera::Camera, entity_manager::EntityManager, lights::Lights, particles::ParticleSystem,
    spawn_system::SpawnManager, wgpu_backend::render_context::RenderContext,
};

pub struct World {
    pub ecs: EntityManager,
    pub camera: Camera,
    pub lights: Lights,
    pub particles: ParticleSystem,
    pub spawn_manager: SpawnManager,
}

impl World {
    pub fn new(rdr_ctx: &RenderContext) -> Self {
        let ecs = EntityManager::new(10_000, rdr_ctx);
        let camera = Camera::new();
        let lights = Lights::new(50);
        let particles = ParticleSystem::new("config/emitter_data.json");
        let spawn_manager = SpawnManager::new("config/round_data.json");

        Self {
            ecs,
            camera,
            lights,
            particles,
            spawn_manager,
        }
    }
}
