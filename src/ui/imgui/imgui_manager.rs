use imgui::Key;
use imgui_wgpu::{RenderData, RendererConfig};
use winit::{
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::{
    camera::Camera,
    config::entity_config::UiEntityTypeHelper,
    entity_manager::EntityManager,
    enums_types::CameraState,
    input::InputState,
    lights::Lights,
    particles::ParticleSystem,
    physics::PhysicsState,
    sound::sound_manager::SoundManager,
    ui::{
        imgui::{
            ability_editor::AbilityEditor, entity_editor::EntityEditor,
            particle_editor::ParticleEditor, player_data::PlayerData,
            weapon_pool_editor::WeaponPoolEditor,
        },
        message_queue::MessageQueue,
    },
};

pub struct ImguiManager {
    pub imgui: imgui::Context,
    pub renderer: imgui_wgpu::Renderer,
    pub entity_editor: EntityEditor,
    pub particle_editor: ParticleEditor,
    pub ability_editor: AbilityEditor,
    pub weapon_pool_editor: WeaponPoolEditor,
    pub _player_data: PlayerData,
}

pub struct PreparedImguiRender<'a> {
    renderer: &'a imgui_wgpu::Renderer,
    draw_data: &'a imgui::DrawData,
    render_data: RenderData,
}

impl PreparedImguiRender<'_> {
    pub fn render(&self, rpass: &mut wgpu::RenderPass<'_>) -> imgui_wgpu::RendererResult<()> {
        self.renderer
            .split_render(self.draw_data, &self.render_data, rpass)
    }
}

impl ImguiManager {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_format: wgpu::TextureFormat,
    ) -> Self {
        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        let renderer_config = RendererConfig {
            texture_format,
            ..RendererConfig::new()
        };
        let renderer = imgui_wgpu::Renderer::new(&mut imgui, device, queue, renderer_config);

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
                new_entity_count: 1,
            },
            particle_editor: ParticleEditor::default(),
            ability_editor: AbilityEditor::default(),
            weapon_pool_editor: WeaponPoolEditor::default(),
            _player_data: PlayerData {},
        }
    }

    pub fn handle_imgui_event(&mut self, event: &WindowEvent) {
        let io = self.imgui.io_mut();

        match event {
            // Mouse buttons
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = *state == ElementState::Pressed;
                match button {
                    MouseButton::Left => io.mouse_down[0] = pressed,
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

    pub fn prepare_render(
        &mut self,
        width: f32,
        height: f32,
        delta: f32,
        lm: &mut Lights,
        sm: &mut SoundManager,
        camera: &Camera,
        em: &mut EntityManager,
        ps: &mut PhysicsState,
        input: &mut InputState,
        particles: &mut ParticleSystem,
        message_queue: &mut MessageQueue,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<PreparedImguiRender<'_>> {
        {
            let io = self.imgui.io_mut();
            io.display_size = [width, height];
            io.delta_time = delta;
        }

        {
            let ui = self.imgui.frame();

            // BUILD WINDOWS
            if camera.move_state == CameraState::Locked {
                self.entity_editor
                    .draw(ui, em, ps, lm, sm, input, &[width, height]);
                self.particle_editor.draw(
                    ui,
                    em,
                    ps,
                    lm,
                    sm,
                    input,
                    &[width, height],
                    particles,
                    delta,
                    message_queue,
                );
                self.ability_editor.draw(ui, em, &[width, height]);
                self.weapon_pool_editor.draw(ui, em, &[width, height]);
            } else {
                //self.player_data.draw(ui, em, &[width, height]);
            }
        }

        let draw_data = self.imgui.render();

        if draw_data.total_vtx_count > 0 {
            let render_data = self.renderer.prepare(draw_data, None, queue, device);
            Some(PreparedImguiRender {
                renderer: &self.renderer,
                draw_data,
                render_data,
            })
        } else {
            None
        }
    }
}
