use crate::{
    abilities::{AbilityDefinition, AbilityId},
    config::Config,
    entity_manager::EntityManager,
};
use imgui::Ui;

pub struct AbilityEditor {
    pub staged_name: String,
    pub staged_cooldown: f32,
    pub staged_description: String,
    pub staged_icon: String,
}

impl Default for AbilityEditor {
    fn default() -> Self {
        Self {
            staged_name: String::new(),
            staged_cooldown: 0.0,
            staged_description: String::new(),
            staged_icon: String::new(),
        }
    }
}

impl AbilityEditor {
    pub fn draw(&mut self, ui: &Ui, em: &mut EntityManager, _size: &[f32; 2]) {
        ui.window("Ability Editor")
            .size([400.0, 300.0], imgui::Condition::FirstUseEver)
            .position([0.0, 50.0], imgui::Condition::FirstUseEver)
            .collapsed(true, imgui::Condition::FirstUseEver)
            .build(|| {
                let next_id = em.abilities_config.get_next_id();
                ui.text(format!("Next Ability ID: {}", next_id));

                ui.input_text("Name", &mut self.staged_name).build();
                ui.input_float("Cooldown", &mut self.staged_cooldown)
                    .build();
                ui.input_text("Description", &mut self.staged_description)
                    .build();
                ui.input_text("Icon Path", &mut self.staged_icon).build();

                ui.separator();

                if ui.button("Save Ability") {
                    if !self.staged_name.is_empty() {
                        let new_ability = AbilityDefinition {
                            id: next_id,
                            name: self.staged_name.clone(),
                            cooldown: self.staged_cooldown,
                            description: self.staged_description.clone(),
                            icon: self.staged_icon.clone(),
                            animation: "".to_string(),

                            payload: None,
                        };

                        em.abilities_config.abilities.push(new_ability);
                        em.abilities_config
                            .save_to_file("config/abilities_config.json");

                        // reset fields
                        self.staged_name.clear();
                        self.staged_cooldown = 0.0;
                        self.staged_description.clear();
                        self.staged_icon.clear();
                    }
                }

                ui.separator();
                ui.text("Existing Abilities:");
                let mut ability_ids: Vec<AbilityId> =
                    em.abilities_config.abilities.iter().map(|k| k.id).collect();
                ability_ids.sort_unstable();

                let mut id_to_remove = None;
                for id in ability_ids {
                    if let Some(def) = em.abilities_config.get(id) {
                        ui.text(format!("{}: {} ({:.1}s)", id, def.name, def.cooldown));
                        ui.same_line();
                        if ui.button(format!("Remove##{}", id)) {
                            id_to_remove = Some(id);
                        }
                    }
                }

                if let Some(id) = id_to_remove {
                    em.abilities_config.remove_by_id_unordered(id);
                    em.abilities_config
                        .save_to_file("config/abilities_config.json");
                }
            });
    }
}
