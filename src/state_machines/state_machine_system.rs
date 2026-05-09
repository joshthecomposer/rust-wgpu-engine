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
    // Tick and expire knockbacks (used to gate locomotion/root-motion writes).
    if em.knockbacks.len() > 0 {
        let expired: Vec<usize> = em
            .knockbacks
            .iter_mut()
            .filter_map(|entry| {
                let id = entry.key();
                let kb = entry.value_mut();
                kb.ttl -= dt;
                (kb.ttl <= 0.0).then_some(id)
            })
            .collect();

        for id in expired {
            em.knockbacks.remove(id);
        }
    }

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
