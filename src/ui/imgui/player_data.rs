use imgui::Ui;

use crate::entity_manager::EntityManager;

pub struct PlayerData {}

impl PlayerData {
    pub fn draw(&mut self, ui: &mut Ui, em: &mut EntityManager, size: &[f32; 2]) {
        ui.window("Player Data")
            .size([500.0, size[1]], imgui::Condition::FirstUseEver)
            .position([0.0, 0.0], imgui::Condition::FirstUseEver)
            .build(|| {
                let maybe_player_entry = em.factions.iter().find(|e| *e.value() == "Player");
                // ===================== Player =====================
                match maybe_player_entry {
                    Some(player) => {
                        let player_id = player.key();
                        let transform = em.transforms.get(player_id).unwrap();
                        let controller = em.player_controllers.get(player_id).unwrap();
                        let animator = em.animators.get(player_id).unwrap();
                        let active_item = em.active_items.get(player_id).unwrap();

                        ui.separator();
                        ui.text("Player Data:");
                        ui.separator();
                        ui.text(format!(
                            "Position: x: {} y: {} z: {}",
                            transform.position.x, transform.position.y, transform.position.z
                        ));
                        ui.text(format!("Player State: {}", controller.state));
                        ui.text(format!("Attack State: {}", controller.attack_state));
                        ui.text(format!("Current Animation: {}", animator.current_animation));
                        if let Some(weapon_id) = active_item.right_hand {
                            let hitset = em.hitsets.get(weapon_id).unwrap();

                            for hit in hitset.iter() {
                                ui.text(format!("Hitset: {:?}", hit));
                            }
                        }
                    }
                    None => (),
                }
            });
    }
}
