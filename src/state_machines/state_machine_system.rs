use crate::{
    command_buffer::CommandBuffer, entity_manager::EntityManager, input::InputState,
    state_machines::player::orchestrator::player_state_orchestrator,
};

pub fn update(em: &mut EntityManager, input: &InputState, cmds: &mut CommandBuffer, dt: f32) {
    player_state_orchestrator(em, input, cmds, dt);
}
