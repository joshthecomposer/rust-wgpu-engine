use std::borrow::Cow;

use glam::{Mat4, Quat, Vec3};
use glfw::{Action, MouseButton, PWindow, WindowEvent};
use imgui::{sys::{ImGuiKey, ImGuiKey_Backspace}, Drag, Io};

use crate::{animation::animation::Animator, camera::Camera, config::{entity_config::{EntityTypeHelper, UiEntityTypeHelper}, world_data::{EntityInstance, WorldData}}, entity_manager::EntityManager, enums_types::{CameraState, EntityType, Faction, SoundType}, gl_call, input::InputState, lights::Lights, physics::PhysicsState, renderer::Renderer, sound::sound_manager::SoundManager, util::data_structure::HashMapGetPairMut};

pub struct ImguiManager {
    pub imgui: imgui::Context,
    pub renderer: imgui_opengl_renderer::Renderer,
    pub entity_type_index: usize,
    pub weapon_type_index: usize,
    pub faction_index: usize,
    pub hitbox_type_index: usize,
    pub create_mode: bool,
    pub include_weapon: bool,
    pub remove_entity_type_idx: usize,

    pub new_archetype: UiEntityTypeHelper,
}

impl ImguiManager {
    pub fn new(window: &mut PWindow) -> Self {
        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);
        let renderer = imgui_opengl_renderer::Renderer::new(&mut imgui, |s| {
             window.get_proc_address(s) as *const _
         });
        Self {
            imgui,
            renderer,
            entity_type_index: 0,
            weapon_type_index: 0,
            hitbox_type_index: 0,
            faction_index: 0,
            create_mode: false,
            include_weapon: false,
            remove_entity_type_idx: 0,

            new_archetype: UiEntityTypeHelper::default(),
        }
    }

    pub fn handle_imgui_event(&mut self, event: &WindowEvent) {
        let io = self.imgui.io_mut();
        match *event {
            // Mouse Buttons
            WindowEvent::MouseButton(btn, action, _) => {
                let pressed = action != Action::Release;
                match btn {
                    MouseButton::Button1 => {
                        io.mouse_down[0] = pressed;
                    },
                    MouseButton::Button2 => io.mouse_down[1] = pressed,
                    MouseButton::Button3 => io.mouse_down[2] = pressed,
                    _ => {}
                }
            }
            // Mouse Position
            WindowEvent::CursorPos(x, y) => {
                io.mouse_pos = [x as f32, y as f32];
            }
            // Scroll Wheel
            WindowEvent::Scroll(_x, scroll_y) => {
                io.mouse_wheel = scroll_y as f32;
            }
            // Text input
            WindowEvent::Char(ch) => {
                io.add_input_character(ch);
            }
            // Key press/release
            WindowEvent::Key(_key, _, action, _mods) => {
                let pressed = action != Action::Release;
                match _key {

                    // this is where we map keys from glfw to imgui if the keys don't work
                    glfw::Key::Backspace => io.add_key_event(imgui::Key::Backspace, pressed),
                    _ => {}
                }
            }

            _ => {}
        }
    }

    pub fn draw(&mut self, window: &mut PWindow, width: f32, height: f32, delta: f32, lm: &mut Lights, rdr: &mut Renderer, sm: &mut SoundManager, camera: &Camera, em: &mut EntityManager, ps: &mut PhysicsState, input: &mut InputState) {
        {
            let io = self.imgui.io_mut();
            io.display_size = [width, height];
            io.delta_time   = delta;
        }

        {

            // BUILD WINDOWS
            let ui = self.imgui.frame();
            if camera.move_state == CameraState::Locked {
                ui.window("Editor")
                    .size([500.0, 800.0], imgui::Condition::FirstUseEver)
                    .position([0.0, 0.0], imgui::Condition::FirstUseEver)
                    .build(|| {
                        // Hoist the vars!
                        let mut entity_types: Vec<String> = em.entity_type_register
                            .keys()
                            .map(|k| k.clone())
                            .collect();

                        let factions: Vec<&str> = em.faction_register
                            .iter()
                            .map(|k| k.as_str())
                            .collect();

                        let hb_types: Vec<&str> = vec!["Cylinder", "Pill", "BoxDim", "Sphere", "Mesh", "BoundingBox"];

                        // ===================== Lights =====================
                        ui.separator();
                        ui.text("Controls for Various Lights");
                        ui.separator();

                        if ui.slider("Dir Light X", -1.0, 1.0, &mut lm.dir_light.direction.x) {
                            lm.dir_light.view_pos.x = lm.dir_light.direction.x * lm.dir_light.distance;
                        };
                        if ui.slider("Dir Light Y", -1.0, 1.0, &mut lm.dir_light.direction.y) {
                            lm.dir_light.view_pos.y = lm.dir_light.direction.y * lm.dir_light.distance;
                        };
                        if ui.slider("Dir Light Z", -1.0, 1.0, &mut lm.dir_light.direction.z) {
                            lm.dir_light.view_pos.z = lm.dir_light.direction.z * lm.dir_light.distance;
                        };
                        if ui.slider("Dir Light distance", 0.0, 100.0, &mut lm.dir_light.distance) {
                        };

                        ui.checkbox("Shadow Debug", &mut rdr.shadow_debug);

                        if ui.slider("Ortho Near", 0.0, 10.0, &mut lm.near) {
                        };
                        if ui.slider("Ortho Far", 0.0, 500.0, &mut lm.far) {
                        };
                        if ui.slider("Bounds", 0.0, 100.0, &mut lm.bounds) {
                        };

                        if Drag::new("Bias Scalar")
                            .speed(0.0001)
                            .display_format("%.6f")
                            .build(ui, &mut lm.bias_scalar)
                        {
                        }

                        lm.dir_light.view_pos.x = lm.dir_light.direction.x * lm.dir_light.distance;
                        lm.dir_light.view_pos.y = lm.dir_light.direction.y * lm.dir_light.distance;
                        lm.dir_light.view_pos.z = lm.dir_light.direction.z * lm.dir_light.distance;

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

                        // ===================== Entity Editing =====================
                        ui.separator();
                        ui.text("Entity Editing");
                        ui.separator();

                        if ui.button("Save Entity State") {
                            em.serialize_entity_data();
                        }

                        for i in em.selected.iter() {
                            if let Some(trans) = em.transforms.get_mut(*i) {
                                ui.text(format!(
                                    "Entity: {}, Type: {}",
                                    i,
                                    em.entity_types.get(*i).unwrap()
                                ));

                                let mut position = [trans.position.x, trans.position.y, trans.position.z];
                                let mut rotation = [trans.rotation.x, trans.rotation.y, trans.rotation.z, trans.rotation.w];

                                // position
                                if Drag::new("Position").speed(0.1).build_array(ui, &mut position) {};
                                trans.position = Vec3::from(position);

                                // rotation
                                if Drag::new("Rotation").speed(0.1).build_array(ui, &mut rotation) {};
                                trans.rotation = Quat::from_slice(&rotation);
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

                        ui.combo(
                            "Factions",
                            &mut self.faction_index,
                            &factions,
                            |s| Cow::Borrowed(*s),
                        );

                        ui.combo(
                            "Weapon",
                            &mut self.weapon_type_index,
                            &entity_types,
                            |s| Cow::Borrowed(&s),
                        );

                        if ui.checkbox("Include Weapon", &mut self.include_weapon) {
                            println!("Clicked Create Mode");
                        }

                        let selected_type = &entity_types[self.entity_type_index];
                        let selected_faction = factions[self.faction_index];

                        let weapons = if self.include_weapon {
                            Some(vec![
                                EntityInstance {
                                    entity_type: entity_types[self.weapon_type_index].clone(),
                                    faction: "Item".to_string(),
                                    position: Vec3::splat(0.0),
                                    rotation: Quat::IDENTITY,
                                    base_speed: None,
                                    jump_height: None,
                                    health: None,
                                    weapons: None,
                                    cleanup_timer: None,
                                }
                            ])
                        } else {
                            None
                        };

                        if self.create_mode {
                            let instance = EntityInstance {
                                entity_type: selected_type.to_string(),
                                faction: selected_faction.to_string(),
                                position: input.ray_just_hit,
                                rotation: Quat::IDENTITY,
                                weapons,
                                base_speed: Some(4.5),
                                jump_height: Some(1.0),
                                health: Some(100.0),
                                cleanup_timer: None,
                            };

                            let parent_id = em.create_entity(
                                &instance,
                                ps,
                            );
                            em.populate_inventory(parent_id, &instance, ps);
                            self.create_mode = false;
                        }

                        if ui.checkbox("Create Mode", &mut self.create_mode) {
                            println!("Clicked Create Mode");
                        }

                        // ===================== Create A New Entity Type =====================
                        ui.separator();
                        ui.text("Create a new Entity Type");
                        ui.separator();

                        ui.input_text("Entity Type", &mut self.new_archetype.entity_type)
                            .build();
                        
                        if Drag::new("Rot Correction").speed(0.1).build_array(ui, &mut self.new_archetype.rot_correction) {};

                        if Drag::new("Scale Correction").speed(0.1).build_array(ui, &mut self.new_archetype.scale_correction) {};

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
                            },
                            "BoxDim" => {
                                self.new_archetype.r = 0.0;
                                self.new_archetype.h = 0.0;

                                ui.input_float("Half X", &mut self.new_archetype.hx)
                                    .build();

                                ui.input_float("Half Y", &mut self.new_archetype.hy)
                                    .build();

                                ui.input_float("Half Z", &mut self.new_archetype.hz)
                                    .build();
                            },
                            "Sphere" => {
                                self.new_archetype.h = 0.0;
                                self.new_archetype.hx = 0.0;
                                self.new_archetype.hy = 0.0;
                                self.new_archetype.hz = 0.0;

                                ui.input_float("HB Radius", &mut self.new_archetype.r)
                                    .build();
                            },
                            "Mesh" | "BoundingBox" => {
                                self.new_archetype.r = 0.0;
                                self.new_archetype.h = 0.0;
                                self.new_archetype.hx = 0.0;
                                self.new_archetype.hy = 0.0;
                                self.new_archetype.hz = 0.0;
                            },
                            _=> {},
                        }

                        self.new_archetype.hitbox = hb_types[self.hitbox_type_index].to_string();

                        if ui.button("Save New Entity Type") {
                            em.register_new_entity_type(&self.new_archetype);
                        }

                        ui.separator();
                        ui.text("Remove an Entity Type");
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
                    });
            }
        }

        let draw_data = self.imgui.render();

        if draw_data.total_vtx_count > 0 {
            self.renderer.render(&mut self.imgui);
        }

    }

}

