use crate::{camera::Camera, entity_manager::EntityManager, enums_types::Faction, input::InputState, particles::ParticleSystem, physics::PhysicsState, sound::sound_manager::SoundManager, state_machines::{enemy_state_machine::enemy_sim_state_machine, player_state_machine::player_state_machine}};

pub fn update(
    em: &mut EntityManager, 
    dt: f32, 
    particles: &mut ParticleSystem, 
    input: &InputState, 
    ps: &mut PhysicsState, 
    sm: &mut SoundManager,
    camera: &Camera,
) {
    // COMMON DATA BETWEEN MACHINES
    let player_id = em.factions.iter().find(|e| *e.value() == Faction::Player).unwrap().key();
    player_state_machine(em, dt, input, ps, sm, particles, camera);
        
    // TODO: gather entity IDs once somewhere and use for the entire game loop?
    let enemy_ids = em.factions.iter().filter(|e| *e.value() == Faction::Enemy).map(|e| e.key()).collect::<Vec<usize>>();
    for id in enemy_ids.iter() {
        enemy_sim_state_machine(*id, em, dt, particles, ps, input, player_id);
    }
}
