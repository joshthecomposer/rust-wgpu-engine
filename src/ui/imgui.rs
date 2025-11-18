use std::borrow::Cow;

use glam::{Mat4, Quat, Vec3};
use glfw::{Action, MouseButton, PWindow, WindowEvent};
use imgui::Drag;

use crate::{animation::animation::Animator, camera::Camera, config::world_data::{EntityInstance, WorldData}, entity_manager::EntityManager, enums_types::{CameraState, EntityType, Faction, SoundType}, gl_call, input::InputState, lights::Lights, physics::PhysicsState, renderer::Renderer, sound::sound_manager::SoundManager, util::data_structure::HashMapGetPairMut};

pub struct ImguiManager {
    pub imgui: imgui::Context,
    pub renderer: imgui_opengl_renderer::Renderer,
    pub entity_type_index: usize,
    pub weapon_type_index: usize,
    pub faction_index: usize,
    pub create_mode: bool,
    pub include_weapon: bool,
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
            faction_index: 0,
            create_mode: false,
            include_weapon: false,
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
                let _pressed = action != Action::Release;
                // If i want to track ImGui’s internal key map, we can do:
                // io.keys_down[imgui_key_index] = pressed;
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
                        // ===================== Lights =====================
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
                        ui.text("Placing Entities");
                        ui.separator();

                        let types: Vec<&str> = em.entity_type_register
                            .keys()
                            .map(|k| k.as_str())
                            .collect();

                        ui.combo(
                            "Entity Types",
                            &mut self.entity_type_index,
                            &types,
                            |s| Cow::Borrowed(*s),
                        );

                        let factions: Vec<&str> = em.faction_register
                            .iter()
                            .map(|k| k.as_str())
                            .collect();

                        ui.combo(
                            "Factions",
                            &mut self.faction_index,
                            &factions,
                            |s| Cow::Borrowed(*s),
                        );

                        ui.combo(
                            "Weapon",
                            &mut self.weapon_type_index,
                            &types,
                            |s| Cow::Borrowed(*s),
                        );

                        if ui.checkbox("Include Weapon", &mut self.include_weapon) {
                            println!("Clicked Create Mode");
                        }

                        let selected_type = types[self.entity_type_index];
                        let selected_faction = factions[self.faction_index];

                        let weapons = if self.include_weapon {
                            Some(vec![
                                EntityInstance {
                                    entity_type: types[self.weapon_type_index].to_string(),
                                    faction: "Item".to_string(),
                                    position: Vec3::splat(0.0),
                                    rotation: Quat::IDENTITY,
                                    base_speed: None,
                                    jump_height: None,
                                    health: None,
                                    weapons: None,
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
                    });
            }
        }

        let draw_data = self.imgui.render();

        if draw_data.total_vtx_count > 0 {
            self.renderer.render(&mut self.imgui);
        }

    }

}

