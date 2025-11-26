use std::borrow::Cow;

use glam::{Mat4, Quat, Vec3};
use glfw::{Action, MouseButton, PWindow, WindowEvent};
use imgui::{sys::{ImGuiKey, ImGuiKey_Backspace}, Drag, Io, Ui};

use crate::{animation::animation::Animator, camera::Camera, config::{entity_config::{EntityTypeHelper, UiEntityTypeHelper}, world_data::{EntityInstance, WorldData}}, entity_manager::EntityManager, enums_types::{CameraState, EntityType, Faction, SoundType}, gl_call, input::InputState, lights::Lights, particles::ParticleSystem, physics::PhysicsState, renderer::Renderer, sound::sound_manager::SoundManager, ui::imgui::{entity_editor::EntityEditor, particle_editor::ParticleEditor, player_data::PlayerData}, util::data_structure::HashMapGetPairMut};

pub struct ImguiManager {
    pub imgui: imgui::Context,
    pub renderer: imgui_opengl_renderer::Renderer,
    pub entity_editor: EntityEditor,
    pub particle_editor: ParticleEditor,
    pub player_data: PlayerData,
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
            entity_editor: EntityEditor {
                entity_type_index: 0,
                weapon_type_index: 0,
                hitbox_type_index: 0,
                faction_index: 0,
                create_mode: false,
                include_weapon: false,
                remove_entity_type_idx: 0,
                new_archetype: UiEntityTypeHelper::default(),
                new_faction: String::new(),
                base_speed: 0.0,
            },
            particle_editor: ParticleEditor::default(),
            player_data: PlayerData {},
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

    pub fn draw(
        &mut self, 
        window: &mut PWindow, 
        width: f32, 
        height: f32, 
        delta: f32, 
        lm: &mut Lights, 
        rdr: &mut Renderer, 
        sm: &mut SoundManager, 
        camera: &Camera, 
        em: &mut EntityManager, 
        ps: &mut PhysicsState, 
        input: &mut InputState,
        particles: &mut ParticleSystem,
    ) {
        {
            let io          = self.imgui.io_mut();
            io.display_size = [width, height];
            io.delta_time   = delta;
        }

        {
            let ui = self.imgui.frame();

            // BUILD WINDOWS
            if camera.move_state == CameraState::Locked {
                self.entity_editor.draw(ui, em, ps, rdr, lm, sm, input, &[width, height]);
                self.particle_editor.draw(ui, em, ps, rdr, lm, sm, input, &[width, height], particles, delta);
            } else {
                self.player_data.draw(ui, em, &[width, height]);
            }
        }

        let draw_data = self.imgui.render();

        if draw_data.total_vtx_count > 0 {
            self.renderer.render(&mut self.imgui);
        }

    }
}
