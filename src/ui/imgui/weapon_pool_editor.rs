use std::borrow::Cow;
use imgui::Ui;
use crate::{
    abilities::{AbilityId, WeaponTypeAbilities},
    entity_manager::EntityManager,
    config::Config,
};

pub struct WeaponPoolEditor {
    pub selected_entity_type_idx: usize,
    pub selected_category_idx: usize,
    pub new_category_name: String,
    
    // For editing a category
    pub edit_category_idx: usize,
    pub m1_idx: usize,
    pub m2_idx: usize,
    pub shift_idx: usize,
    
    // For skill pools
    pub skill_to_add_idx: usize,
    pub ultimate_to_add_idx: usize,
}

impl Default for WeaponPoolEditor {
    fn default() -> Self {
        Self {
            selected_entity_type_idx: 0,
            selected_category_idx: 0,
            new_category_name: String::new(),
            edit_category_idx: 0,
            m1_idx: 0,
            m2_idx: 0,
            shift_idx: 0,
            skill_to_add_idx: 0,
            ultimate_to_add_idx: 0,
        }
    }
}

impl WeaponPoolEditor {
    pub fn draw(
        &mut self,
        ui: &Ui,
        em: &mut EntityManager,
        _size: &[f32; 2],
    ) {
        ui.window("Weapon Pool Editor")
            .size([500.0, 400.0], imgui::Condition::FirstUseEver)
            .position([0.0, 25.0], imgui::Condition::FirstUseEver)
            .collapsed(true, imgui::Condition::FirstUseEver)
            .build(|| {
                // --- Section 1: Entity Type Mapping ---
                ui.text("Entity Type to Weapon Category Mapping");
                ui.separator();
                
                let mut entity_types: Vec<String> = em.entity_type_register.keys().cloned().collect();
                entity_types.sort_unstable();
                
                ui.combo("Entity Type", &mut self.selected_entity_type_idx, &entity_types, |s| Cow::Borrowed(s));
                
                let mut categories: Vec<String> = em.weapon_pools_config.weapon_types.keys().cloned().collect();
                categories.sort_unstable();
                
                if let Some(entity_type) = entity_types.get(self.selected_entity_type_idx) {
                    let current_cat = em.weapon_pools_config.entity_type_mapping.get(entity_type).cloned().unwrap_or_else(|| "None".to_string());
                    ui.text(format!("Current Mapping: {}", current_cat));
                    
                    ui.combo("Weapon Category", &mut self.selected_category_idx, &categories, |s| Cow::Borrowed(s));
                    
                    if ui.button("Apply Mapping") {
                        if let Some(cat) = categories.get(self.selected_category_idx) {
                            em.weapon_pools_config.entity_type_mapping.insert(entity_type.clone(), cat.clone());
                            em.weapon_pools_config.save_to_file("config/weapon_pools_config.json");
                        }
                    }
                }
                
                ui.spacing();
                ui.separator();
                
                // --- Section 2: Weapon Category Editor ---
                ui.text("Weapon Category Definitions");
                ui.spacing();
                
                ui.input_text("New Category Name", &mut self.new_category_name).build();
                if ui.button("Create Category") {
                    if !self.new_category_name.is_empty() && !em.weapon_pools_config.weapon_types.contains_key(&self.new_category_name) {
                        em.weapon_pools_config.weapon_types.insert(self.new_category_name.clone(), WeaponTypeAbilities {
                            m1: 0,
                            m2: 0,
                            shift: 0,
                            skill_pool: vec![],
                            ultimate_pool: vec![],
                        });
                        em.weapon_pools_config.save_to_file("config/weapon_pools_config.json");
                        self.new_category_name.clear();
                    }
                }
                
                ui.spacing();
                
                ui.combo("Edit Category", &mut self.edit_category_idx, &categories, |s| Cow::Borrowed(s));
                
                if let Some(category_name) = categories.get(self.edit_category_idx) {
                    if let Some(weapon_type) = em.weapon_pools_config.weapon_types.get_mut(category_name) {
                        ui.text(format!("Editing: {}", category_name));
                        
                        let mut abilities: Vec<(AbilityId, String)> = em.abilities_config.abilities.iter()
                            .map(|(k, v)| (k.parse().unwrap(), v.name.clone()))
                            .collect();
                        abilities.sort_by_key(|a| a.0);
                        
                        let ability_names: Vec<String> = abilities.iter().map(|a| format!("{}: {}", a.0, a.1)).collect();
                        
                        // M1
                        self.m1_idx = abilities.iter().position(|a| a.0 == weapon_type.m1).unwrap_or(0);
                        if ui.combo("M1 Attack", &mut self.m1_idx, &ability_names, |s| Cow::Borrowed(s)) {
                            weapon_type.m1 = abilities[self.m1_idx].0;
                        }
                        
                        // M2
                        self.m2_idx = abilities.iter().position(|a| a.0 == weapon_type.m2).unwrap_or(0);
                        if ui.combo("M2 Attack", &mut self.m2_idx, &ability_names, |s| Cow::Borrowed(s)) {
                            weapon_type.m2 = abilities[self.m2_idx].0;
                        }
                        
                        // Shift
                        self.shift_idx = abilities.iter().position(|a| a.0 == weapon_type.shift).unwrap_or(0);
                        if ui.combo("Shift Ability", &mut self.shift_idx, &ability_names, |s| Cow::Borrowed(s)) {
                            weapon_type.shift = abilities[self.shift_idx].0;
                        }
                        
                        ui.spacing();
                        ui.text("Skill Pool (Q/E)");
                        let mut skill_pool_to_remove = None;
                        for (i, &skill_id) in weapon_type.skill_pool.iter().enumerate() {
                            let name = em.abilities_config.get(skill_id).map(|a| a.name.as_str()).unwrap_or("Unknown");
                            ui.text(format!("{}: {}", skill_id, name));
                            ui.same_line();
                            if ui.button(format!("Remove##skill_{}", i)) {
                                skill_pool_to_remove = Some(i);
                            }
                        }
                        if let Some(i) = skill_pool_to_remove {
                            weapon_type.skill_pool.remove(i);
                        }
                        
                        ui.combo("Add Skill", &mut self.skill_to_add_idx, &ability_names, |s| Cow::Borrowed(s));
                        if ui.button("Add to Skill Pool") {
                            let id = abilities[self.skill_to_add_idx].0;
                            if !weapon_type.skill_pool.contains(&id) {
                                weapon_type.skill_pool.push(id);
                            }
                        }
                        
                        ui.spacing();
                        ui.text("Ultimate Pool (R)");
                        let mut ult_pool_to_remove = None;
                        for (i, &ult_id) in weapon_type.ultimate_pool.iter().enumerate() {
                            let name = em.abilities_config.get(ult_id).map(|a| a.name.as_str()).unwrap_or("Unknown");
                            ui.text(format!("{}: {}", ult_id, name));
                            ui.same_line();
                            if ui.button(format!("Remove##ult_{}", i)) {
                                ult_pool_to_remove = Some(i);
                            }
                        }
                        if let Some(i) = ult_pool_to_remove {
                            weapon_type.ultimate_pool.remove(i);
                        }
                        
                        ui.combo("Add Ultimate", &mut self.ultimate_to_add_idx, &ability_names, |s| Cow::Borrowed(s));
                        if ui.button("Add to Ultimate Pool") {
                            let id = abilities[self.ultimate_to_add_idx].0;
                            if !weapon_type.ultimate_pool.contains(&id) {
                                weapon_type.ultimate_pool.push(id);
                            }
                        }
                        
                        ui.separator();
                        if ui.button("Save Weapon Pools Config") {
                            em.weapon_pools_config.save_to_file("config/weapon_pools_config.json");
                        }
                    }
                }
            });
    }
}

