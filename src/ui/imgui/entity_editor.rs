use std::borrow::Cow;

use glam::{Quat, Vec3};
use imgui::{Drag, Ui};

use crate::{
    config::{entity_config::UiEntityTypeHelper, world_data::EntityInstance},
    entity_manager::EntityManager,
    enums_types::SoundType,
    input::InputState,
    lights::Lights,
    physics::PhysicsState,
    renderer::Renderer,
    sound::sound_manager::SoundManager,
};

pub struct EntityEditor {
    pub entity_type_index: usize,
    pub weapon_type_index: usize,
    pub faction_index: usize,
    pub hitbox_type_index: usize,
    pub create_mode: bool,
    pub include_weapon: bool,
    pub remove_entity_type_idx: usize,

    pub new_archetype: UiEntityTypeHelper,
    pub new_faction: String,
    pub base_speed: f32,
    pub new_entity_count: i32,
}

impl EntityEditor {
    pub fn draw(
        &mut self,
        ui: &mut Ui,
        em: &mut EntityManager,
        ps: &mut PhysicsState,
        rdr: &mut Renderer,
        lm: &mut Lights,
        sm: &mut SoundManager,
        input: &mut InputState,
        size: &[f32; 2],
    ) {
        ui.window("Entity Editor")
            .size([500.0, size[1]], imgui::Condition::FirstUseEver)
            .position([0.0, 0.0], imgui::Condition::FirstUseEver)
            .collapsed(true, imgui::Condition::FirstUseEver)
            .build(|| {
                // Hoist the vars!
                let mut entity_types: Vec<String> =
                    em.entity_type_register.keys().map(|k| k.clone()).collect();

                entity_types.sort_unstable();

                let mut factions: Vec<String> = em.faction_register.iter().cloned().collect();

                let hb_types: Vec<&str> = vec![
                    "Cylinder",
                    "Pill",
                    "BoxDim",
                    "Sphere",
                    "Mesh",
                    "BoundingBox",
                ];

                let maybe_player_entry = em.factions.iter().find(|e| *e.value() == "Player");
                // ===================== Player =====================
                match maybe_player_entry {
                    Some(player) => {
                        let player_id = player.key();
                        let transform = em.transforms.get(player_id).unwrap();
                        let controller = em.player_controllers.get(player_id).unwrap();
                        let animator = em.animators.get(player_id).unwrap();

                        ui.separator();
                        ui.text("Player Data:");
                        ui.separator();
                        ui.text(format!(
                            "Position: x: {} y: {} z: {}",
                            transform.position.x, transform.position.y, transform.position.z
                        ));
                        //ui.text(format!("Player State: {}", controller.state));
                        //ui.text(format!("Attack State: {}", controller.attack_state));
                        ui.text(format!("Current Animation: {}", animator.current_animation));
                    }
                    None => (),
                }

                // ===================== Lights =====================
                ui.separator();
                ui.text("Controls for Various Lights");
                ui.separator();

                ui.slider("Dir Light X", -1.0, 1.0, &mut lm.dir_light.direction.x);
                ui.slider("Dir Light Y", -1.0, 1.0, &mut lm.dir_light.direction.y);
                ui.slider("Dir Light Z", -1.0, 1.0, &mut lm.dir_light.direction.z);
                ui.slider("Dir Light distance", 0.0, 100.0, &mut lm.dir_light.distance);

                ui.checkbox("Shadow Debug", &mut rdr.shadow_debug);

                ui.slider("Ortho Near", 0.0, 10.0, &mut lm.near);
                ui.slider("Ortho Far", 0.0, 500.0, &mut lm.far);
                ui.slider("Bounds", 0.0, 100.0, &mut lm.bounds);

                if Drag::new("Bias Scalar")
                    .speed(0.0001)
                    .display_format("%.6f")
                    .build(ui, &mut lm.bias_scalar)
                {}

                lm.dir_light.view_pos = lm.dir_light.direction * lm.dir_light.distance;

                // ===================== Sound =====================
                ui.separator();
                ui.text("Controls Fmod Sounds");
                ui.separator();

                if ui.button("Pause") {
                    sm.stop_sound(&SoundType::Music);
                }

                if ui.button("Play") {
                    sm.play_sound_2d(SoundType::Music);
                }

                if ui.slider("Volume", 0.0, 1.0, &mut sm.master_volume) {
                    sm.set_master_volume(&SoundType::Music);
                }

                // ===================== bloom stuff =====================

                if Drag::new("Exposure")
                    .speed(0.01)
                    .build(ui, &mut rdr.exposure)
                {};
                if ui.checkbox("Do HDR", &mut rdr.do_hdr) {};
                if Drag::new("Bloom Strength")
                    .speed(0.01)
                    .build(ui, &mut rdr.bloom_strength)
                {};

                // ===================== Entity Editing =====================
                ui.separator();
                ui.text("Entity Editing");
                ui.separator();

                if ui.button("Save Entity State") {
                    em.serialize_entity_data("config/world_data.json");
                }

                for i in em.selected.iter() {
                    if let Some(trans) = em.transforms.get_mut(*i) {
                        ui.text(format!(
                            "Entity: {}, Type: {}",
                            i,
                            em.entity_types.get(*i).unwrap()
                        ));

                        let mut position = [trans.position.x, trans.position.y, trans.position.z];
                        let mut rotation = [
                            trans.rotation.x,
                            trans.rotation.y,
                            trans.rotation.z,
                            trans.rotation.w,
                        ];

                        // position
                        if Drag::new("Position")
                            .speed(0.1)
                            .build_array(ui, &mut position)
                        {};
                        trans.position = Vec3::from(position);

                        // rotation
                        if Drag::new("Rotation")
                            .speed(0.01)
                            .build_array(ui, &mut rotation)
                        {};
                        trans.rotation = Quat::from_slice(&rotation);
                    }
                }

                // ===================== Creating a Faction =====================

                {
                    ui.separator();
                    ui.text("Create A New Faction");
                    ui.separator();

                    ui.input_text("New Faction Name", &mut self.new_faction)
                        .build();

                    if ui.button("Save Faction") {
                        em.register_new_faction(&self.new_faction);
                        em.serialize_faction_register();
                        factions.push(self.new_faction.to_string());
                    }
                }

                // ===================== Placing Entities =====================
                ui.separator();
                ui.text("Placing Entities");
                ui.separator();

                ui.combo(
                    "Entity Types",
                    &mut self.entity_type_index,
                    &entity_types,
                    |s| Cow::Borrowed(&s),
                );

                ui.combo("Factions", &mut self.faction_index, &factions, |s| {
                    Cow::Borrowed(&s)
                });

                ui.combo("Weapon", &mut self.weapon_type_index, &entity_types, |s| {
                    Cow::Borrowed(&s)
                });

                if ui.checkbox("Include Weapon", &mut self.include_weapon) {
                    println!("Clicked Create Mode");
                }

                let selected_type = &entity_types[self.entity_type_index];
                let selected_faction = &factions[self.faction_index];

                ui.input_float("Base Speed", &mut self.base_speed).build();

                let speed = if self.base_speed <= 0.0 {
                    None
                } else {
                    Some(self.base_speed)
                };

                if self.create_mode {
                    for _ in 0..self.new_entity_count {
                        if selected_faction == "None" {
                            let instance = EntityInstance {
                                entity_type: selected_type.to_string(),
                                faction: None,
                                position: input.ray_pos,
                                rotation: Quat::IDENTITY,
                                weapons: None,
                                base_speed: speed,
                                jump_height: Some(1.0),
                                health: Some(100.0),
                                max_health: Some(100.0),
                                mana: Some(100.0),
                                max_mana: Some(100.0),
                                level: Some(1),
                                name: None,
                                cleanup_timer: None,
                                pickup_range: None,
                            };
                            em.create_meshless_entity(&instance);
                            self.create_mode = false;

                            continue;
                        }

                        let weapons = if self.include_weapon {
                            Some(vec![EntityInstance {
                                entity_type: entity_types[self.weapon_type_index].clone(),
                                faction: Some("Item".to_string()),
                                position: Vec3::splat(0.0),
                                rotation: Quat::IDENTITY,
                                base_speed: None,
                                jump_height: None,
                                health: None,
                                max_health: None,
                                mana: None,
                                max_mana: None,
                                level: None,
                                name: None,
                                weapons: None,
                                cleanup_timer: None,
                                pickup_range: None,
                            }])
                        } else {
                            None
                        };
                        let instance = EntityInstance {
                            entity_type: selected_type.to_string(),
                            faction: Some(selected_faction.to_string()),
                            position: input.ray_pos,
                            rotation: Quat::IDENTITY,
                            weapons,
                            base_speed: speed,
                            jump_height: Some(1.0),
                            health: Some(100.0),
                            max_health: Some(100.0),
                            mana: Some(100.0),
                            max_mana: Some(100.0),
                            level: Some(1),
                            name: None,
                            cleanup_timer: None,
                            pickup_range: None,
                        };

                        let parent_id = em.create_mesh_entity(&instance, ps);
                        em.populate_inventory(parent_id, &instance, ps);
                        self.create_mode = false;
                    }
                }

                ui.input_int("How many to spawn?", &mut self.new_entity_count)
                    .build();

                if ui.checkbox("Create Mode", &mut self.create_mode) {
                    println!("Clicked Create Mode");
                }

                // ===================== Create A New Entity Type =====================
                ui.separator();
                ui.text("Create a new Entity Type");
                ui.separator();

                ui.input_text("Entity Type", &mut self.new_archetype.entity_type)
                    .build();

                if Drag::new("Rot Correction")
                    .speed(0.1)
                    .build_array(ui, &mut self.new_archetype.rot_correction)
                {};

                if Drag::new("Scale Correction")
                    .speed(0.1)
                    .build_array(ui, &mut self.new_archetype.scale_correction)
                {};

                ui.input_text("Model Data Path", &mut self.new_archetype.mesh_path)
                    .build();

                ui.input_text("Texture Path", &mut self.new_archetype.texture_path)
                    .build();

                ui.input_float("Aggro Range", &mut self.new_archetype.aggro_range)
                    .build();

                ui.input_float("Total Mass", &mut self.new_archetype.total_mass)
                    .build();

                ui.combo(
                    "Hitbox Types",
                    &mut self.hitbox_type_index,
                    &hb_types,
                    |s| Cow::Borrowed(*s),
                );

                match hb_types[self.hitbox_type_index] {
                    "Cylinder" | "Pill" => {
                        self.new_archetype.hx = 0.0;
                        self.new_archetype.hy = 0.0;
                        self.new_archetype.hz = 0.0;

                        ui.input_float("HB Radius", &mut self.new_archetype.r)
                            .build();

                        ui.input_float("HB Height", &mut self.new_archetype.h)
                            .build();
                    }
                    "BoxDim" => {
                        self.new_archetype.r = 0.0;
                        self.new_archetype.h = 0.0;

                        ui.input_float("Half X", &mut self.new_archetype.hx).build();

                        ui.input_float("Half Y", &mut self.new_archetype.hy).build();

                        ui.input_float("Half Z", &mut self.new_archetype.hz).build();
                    }
                    "Sphere" => {
                        self.new_archetype.h = 0.0;
                        self.new_archetype.hx = 0.0;
                        self.new_archetype.hy = 0.0;
                        self.new_archetype.hz = 0.0;

                        ui.input_float("HB Radius", &mut self.new_archetype.r)
                            .build();
                    }
                    "Mesh" | "BoundingBox" => {
                        self.new_archetype.r = 0.0;
                        self.new_archetype.h = 0.0;
                        self.new_archetype.hx = 0.0;
                        self.new_archetype.hy = 0.0;
                        self.new_archetype.hz = 0.0;
                    }
                    _ => {}
                }

                self.new_archetype.hitbox = hb_types[self.hitbox_type_index].to_string();

                if ui.button("Save New Entity Type") {
                    em.register_new_entity_type(&self.new_archetype);
                }

                ui.separator();
                ui.text("Remove an Entity Type Permanently");
                ui.separator();

                ui.combo(
                    "Delete a type",
                    &mut self.remove_entity_type_idx,
                    &entity_types,
                    |s| Cow::Borrowed(&s),
                );

                if ui.button("Delete") {
                    em.remove_entity_type_definition(&entity_types[self.remove_entity_type_idx]);
                    entity_types.remove(self.remove_entity_type_idx);
                }

                ui.separator();
                ui.text("Remove Instances of Entities");
                ui.separator();

                if ui.button("Destroy All Enemies") {
                    let ids = em.get_ids_for_faction("Enemy");
                    for id in ids {
                        em.entity_trashcan.push(id);
                    }
                }

                if ui.button("Destroy All Orphaned Weapons") {
                    let ids = em.get_all_orphaned_weapon_ids();
                    for id in ids {
                        em.entity_trashcan.push(id);
                    }
                }
            });
    }
}
