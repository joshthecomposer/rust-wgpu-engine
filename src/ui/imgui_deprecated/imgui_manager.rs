use std::{borrow::Cow, ffi::CString};

use glam::{Mat4, Quat, Vec3};
use glutin::prelude::GlDisplay;
use imgui::{sys::{ImGuiKey, ImGuiKey_Backspace}, Drag, Io, Ui};

use imgui::{Context as ImguiContext, Key};
use winit::{
    event::{
        ElementState, Ime, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent
    },
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{animation::animation::Animator, camera::Camera, config::{entity_config::{EntityTypeHelper, UiEntityTypeHelper}, world_data::{EntityInstance, WorldData}}, entity_manager::EntityManager, enums_types::{CameraState, EntityType, Faction, SoundType}, gl_call, input::InputState, lights::Lights, particles::ParticleSystem, physics::PhysicsState, platform::Platform, renderer::Renderer, sound::sound_manager::SoundManager, ui::imgui_deprecated::{entity_editor::EntityEditor, particle_editor::ParticleEditor, player_data::PlayerData}, util::data_structure::HashMapGetPairMut};

pub struct ImguiManager {
    pub imgui: imgui::Context,
    pub renderer: imgui_opengl_renderer::Renderer,
    pub entity_editor: EntityEditor,
    pub particle_editor: ParticleEditor,
    pub player_data: PlayerData,
}

impl ImguiManager {
    pub fn new(platform: &Platform) -> Self {
        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        let renderer = imgui_opengl_renderer::Renderer::new(&mut imgui, |s| {
            let c_str = CString::new(s).unwrap();
            platform.display.get_proc_address(&c_str) as *const _
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

        match event {
            // Mouse buttons
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = *state == ElementState::Pressed;
                match button {
                    MouseButton::Left =>  io.mouse_down[0] = pressed,
                    MouseButton::Right => io.mouse_down[1] = pressed,
                    MouseButton::Middle => io.mouse_down[2] = pressed,
                    _ => {}
                }
            }

            // Mouse position
            WindowEvent::CursorMoved { position, .. } => {
                io.mouse_pos = [position.x as f32, position.y as f32];
            }

            // Scroll wheel
            WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    MouseScrollDelta::LineDelta(_x, y) => {
                        io.mouse_wheel += *y;
                    }
                    MouseScrollDelta::PixelDelta(p) => {
                        // Roughly convert pixels to "lines"
                        io.mouse_wheel += (p.y as f32) / 120.0;
                    }
                }
            }

            // Text input
            WindowEvent::Ime(Ime::Commit(text)) => {
                for ch in text.chars() {
                    io.add_input_character(ch);
                }
            }
           //WindowEvent::ReceivedCharacter(ch) => {
           //    io.add_input_character(*ch);
           //}

            // Key press / release
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;

                if pressed {
                    if let Some(text) = event.text.as_deref() {
                        for ch in text.chars() {
                            if !ch.is_control() {
                                io.add_input_character(ch);
                            }
                        }
                    }
                }


                if let PhysicalKey::Code(code) = event.physical_key {
                    // Map some keys ImGui cares about
                    match code {
                        KeyCode::Backspace => io.add_key_event(Key::Backspace, pressed),
                        KeyCode::Enter | KeyCode::NumpadEnter => {
                            io.add_key_event(Key::Enter, pressed)
                        }
                        KeyCode::Tab => io.add_key_event(Key::Tab, pressed),
                        KeyCode::Escape => io.add_key_event(Key::Escape, pressed),
                        KeyCode::ArrowLeft => io.add_key_event(Key::LeftArrow, pressed),
                        KeyCode::ArrowRight => io.add_key_event(Key::RightArrow, pressed),
                        KeyCode::ArrowUp => io.add_key_event(Key::UpArrow, pressed),
                        KeyCode::ArrowDown => io.add_key_event(Key::DownArrow, pressed),
                        _ => {}
                    }

                    // Modifiers (CTRL/SHIFT/ALT/SUPER)
                    match code {
                        KeyCode::ControlLeft | KeyCode::ControlRight => {
                            io.add_key_event(Key::ModCtrl, pressed);
                        }
                        KeyCode::ShiftLeft | KeyCode::ShiftRight => {
                            io.add_key_event(Key::ModShift, pressed);
                        }
                        KeyCode::AltLeft | KeyCode::AltRight => {
                            io.add_key_event(Key::ModAlt, pressed);
                        }
                        KeyCode::SuperLeft | KeyCode::SuperRight => {
                            io.add_key_event(Key::ModSuper, pressed);
                        }
                        _ => {}
                    }
                }
            }

            _ => {}
        }
    }

    pub fn draw(
        &mut self, 
        window: &mut Window, 
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
