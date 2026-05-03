use crate::{
    command_buffer::CommandBuffer,
    entity_manager::EntityManager,
    input::InputState,
    state_machines::{
        enemy::{self, bt_system},
        player::orchestrator::player_state_orchestrator,
    },
};

pub fn update(em: &mut EntityManager, input: &InputState, cmds: &mut CommandBuffer, dt: f32) {
    for entry in em.enemy_controllers.iter_mut() {
        let ctrl = entry.value_mut();

        if ctrl.took_damage {
            ctrl.taken_damage_ago += dt;
        }

        if ctrl.taken_damage_ago >= ctrl.taken_damage_ttl {
            ctrl.took_damage = false;
            ctrl.taken_damage_ago = 0.0;
        }
    }

    if let Some(player_id) = em.get_player_id() {
        let ctrl = em.player_controllers.get_mut(player_id).unwrap();

        if ctrl.took_damage {
            ctrl.taken_damage_ago += dt;
        }

        if ctrl.taken_damage_ago >= ctrl.taken_damage_ttl {
            ctrl.took_damage = false;
            ctrl.taken_damage_ago = 0.0;
        }
    }

    player_state_orchestrator(em, input, cmds, dt);

    bt_system::update(em);
    enemy::orchestrator::update(em, cmds, dt);
}
