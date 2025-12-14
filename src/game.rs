use std::path::Path;

use gl::ActiveTexture;
use glutin::surface::GlSurface;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::CursorGrabMode;

use crate::animation::animation_system;
use crate::config::game_config::GameConfig;
use crate::entity_manager::EntityManager;
use crate::enums_types::{CameraState, PhysicsHandle, ShaderType, SoundType, Transform};
use crate::input::{self, InputState};
use crate::physics::PhysicsState;
use crate::platform::{CursorMode, Platform};
use crate::renderer::Renderer;
use crate::sound::sound_manager::SoundManager;
use crate::state_machines::state_machine_system;
use crate::time::Time;
use crate::ui::engine_ui_manager::EngineUiManager;
use crate::ui::game_ui::{do_ui, GameUiContext};
use crate::ui::game_ui_manager::{GameUiManager, PauseMenuContext};
use crate::ui::imgui::imgui_manager::ImguiManager;
use crate::ui::message_queue::{MessageQueue, UiMessage};
use crate::util::data_structure::{HashMapGetPair, HashMapGetPairMut};
use crate::world::World;
use crate::{combat_system, items, movement_system, physics};

pub struct Game {
    pub platform: Platform, // OS/window/events
    time: Time,             // delta time, alpha time, elapsed time
    physics: PhysicsState,
    pub world: World, // ECS, terrain, particles, sim
    renderer: Renderer,
    sound: SoundManager,
    pub input: InputState,
    ui: GameUiContext,
    // imgui_manager: ImguiManager,
    pub paused: bool,
    message_queue: MessageQueue,
    engine_ui: EngineUiManager,
    game_ui: GameUiManager,
    pub should_quit: bool,
    imgui_manager: ImguiManager,
}

impl Game {
    pub fn new(platform: Platform, config: GameConfig) -> Self {
        let start_seconds = 0.0;
        let time = Time::new(60.0, start_seconds);

        let mut physics = PhysicsState::new();
        let mut world = World::new();

        world.ecs.populate_entity_data(&mut physics);

        let renderer = Renderer::new(&platform);
        let sound = SoundManager::new(&config);
        let ui = GameUiContext::new();
        let engine_ui = EngineUiManager::new(platform.fb_width, platform.fb_height);
        // GameUiManager must be created AFTER EngineUiManager (platform must be initialized first)
        let game_ui = GameUiManager::new(platform.fb_width, platform.fb_height);

        let imgui_manager = ImguiManager::new(&platform);

        Self {
            platform,
            time,
            physics,
            world,
            renderer,
            sound,
            input: InputState::new(),
            ui,
            paused: false,
            message_queue: MessageQueue::new(),
            engine_ui,
            game_ui,
            should_quit: false,
            imgui_manager,
        }
    }

    pub fn tick(&mut self, now_seconds: f32) {
        self.time.begin_frame(now_seconds);

        // Mouse lock / cursor mode
        if self.paused 
            || self.world.camera.move_state == CameraState::Locked 
            || self.world.camera.move_state == CameraState::SlintSandbox {
            self.platform.window.set_cursor_visible(true);
            let _ = self.platform.window.set_cursor_grab(CursorGrabMode::None);
            self.platform.cursor_mode = CursorMode::Normal;
        } else {
            self.platform.window.set_cursor_visible(false);
            let _ = self
                .platform
                .window
                .set_cursor_grab(CursorGrabMode::Confined);
            self.platform.cursor_mode = CursorMode::Hidden;
        }

        let mut stepped = false;

        while self.time.should_step() {
            stepped = true;
            self.time.begin_fixed_step();

            {
                for curr in self.world.ecs.transforms.iter() {
                    self.world
                        .ecs
                        .prev_transforms
                        .insert(curr.key(), curr.value().clone());
                }

                for curr in self.world.ecs.collider_transforms.iter() {
                    self.world
                        .ecs
                        .prev_collider_transforms
                        .insert(curr.key(), curr.value().clone());
                }
            }

            {
                let cam = &mut self.world.camera;
                cam.prev_pos = cam.position;
                cam.prev_forward = cam.forward;
                cam.prev_up = cam.up;
                cam.prev_target = cam.target;
            }

            let cam_basis = self.world.camera.basis_for_sim();

            if !self.paused {
                physics::grounding_solver(&mut self.world.ecs, &self.physics);

                state_machine_system::update(
                    &mut self.world.ecs,
                    self.time.fixed_dt,
                    &mut self.world.particles,
                    &self.input,
                    &mut self.physics,
                    &mut self.sound,
                    &self.world.camera,
                );

                items::update(&mut self.world.ecs, &mut self.physics);
                animation_system::update(&mut self.world.ecs, self.time.fixed_dt);
                combat_system::update(
                    &mut self.world.ecs,
                    self.time.fixed_dt,
                    &mut self.physics,
                    &mut self.world.particles,
                );
                self.world.ecs.update(
                    &mut self.sound,
                    &mut self.physics,
                    &mut self.input,
                    self.time.fixed_dt,
                );

                physics::push_weapon_kinematics_from_bones(&mut self.world.ecs, &mut self.physics);
                physics::push_static_kinematics(&self.world.ecs, &mut self.physics);

                match self.world.camera.move_state {
                    CameraState::Third | CameraState::Locked => {
                        movement_system::update(
                            &mut self.world.ecs,
                            self.time.fixed_dt,
                            &cam_basis,
                            &self.input,
                            &mut self.physics,
                        );
                    }
                    _ => {}
                }

                self.physics.step();
            }

            physics::sync_transforms_from_physics(&mut self.world.ecs, &self.physics);

            physics::sync_collider_transforms_with_physics(&mut self.world.ecs, &mut self.physics);

            self.time.end_fixed_step();
        }

        self.time.end_frame();

        self.update();
        self.render();

        if stepped {
            self.input.update();
        }
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        self.engine_ui.handle_window_event(&event, &mut self.input);

        self.imgui_manager.handle_imgui_event(event);

        // Forward events to game UI when paused
        if self.paused {
            self.game_ui.handle_window_event(&event, &mut self.input);
        }

        // Process events for the game (keyboard, mouse, etc.)

        // TODO: Currently we always process game events regardless of UI consumption.
        // however, if Slint UI has focused elements we may want to skip game input
        match event {
            WindowEvent::Resized(size) => {
                self.platform.fb_width = size.width;
                self.platform.fb_height = size.height;
                self.game_ui.resize(size.width, size.height);
            }

            WindowEvent::DroppedFile(path) => {
                let path = Path::new(path);
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    match ext {
                        "txt" => {
                            self.imgui_manager.entity_editor.new_archetype.mesh_path =
                                path.to_string_lossy().into_owned();
                        }
                        "png" | "jpg" | "jpeg" => {
                            self.imgui_manager.entity_editor.new_archetype.texture_path =
                                path.to_string_lossy().into_owned();
                            self.imgui_manager.particle_editor.staged_texture =
                                path.to_string_lossy().into_owned();
                        }
                        _ => {}
                    }
                }
            }

            WindowEvent::CloseRequested => {}

            WindowEvent::CursorMoved { position, .. } => {
                if !self.paused {
                    self.world.camera.process_mouse_input_movement(*position);
                }
                self.input.mouse_pos_current = glam::vec2(position.x as f32, position.y as f32);
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let io = self.imgui_manager.imgui.io();
                if !io.want_capture_mouse {
                    let fb = glam::vec2(
                        self.platform.fb_width as f32,
                        self.platform.fb_height as f32,
                    );

                    input::handle_mouse_input(
                        *button,
                        *state,
                        fb,
                        &self.world.camera,
                        &mut self.world.ecs,
                        &mut self.input,
                        &mut self.physics,
                    );
                }
            }

            WindowEvent::KeyboardInput {
                event:
                KeyEvent {
                    physical_key,
                    state,
                    ..
                },
                ..
            } => {
                if let PhysicalKey::Code(code) = physical_key {
                    let keycode: KeyCode = *code;

                    input::handle_keyboard_input(keycode, *state, &mut self.input);

                    if keycode == KeyCode::Escape && *state == ElementState::Pressed {
                        self.paused = !self.paused;
                    }

                    // F toggles camera mode (Free <-> Third <-> Locked)
                    if keycode == KeyCode::KeyF && *state == ElementState::Pressed {
                        let maybe_player_id = self
                            .world
                            .ecs
                            .factions
                            .iter()
                            .find(|e| *e.value() == "Player");

                        self.world.camera.move_state = match self.world.camera.move_state {
                            CameraState::Free => {
                                if maybe_player_id.is_none() {
                                    CameraState::Locked
                                } else {
                                    CameraState::Third
                                }
                            }
                            CameraState::Third => CameraState::Locked,
                            CameraState::Locked => CameraState::SlintSandbox,
                            CameraState::SlintSandbox => CameraState::Free,
                        };
                    }
                }
            }

            _ => {}
        }
    }

    pub fn update(&mut self) {
        self.world.camera.update(
            &self.world.ecs,
            self.time.dt,
            &self.physics,
            self.time.alpha,
            &self.input,
            self.platform.fb_width as f32 / self.platform.fb_height as f32,
        );
        self.sound.update(&self.world.camera);
        self.world.lights.update(&self.time.dt);
        self.world.particles.update(self.time.dt);

        let msgs = self.message_queue.drain();

        for msg in msgs.iter() {
            match msg {
                UiMessage::WindowShouldClose => {
                    self.should_quit = true;
                },
                UiMessage::RenderStagedEmitters { do_it } => {
                    self.world.particles.render_staged_emitters = *do_it;
                },
                UiMessage::ReloadWorldData => {
                    let mut world = World::new();
                    let mut physics = PhysicsState::new();

                    world.ecs.populate_entity_data(&mut physics);

                    // Cleanup 3d sounds
                    {
                        let keys: Vec<usize> = self.sound.active_3d_sounds.keys().cloned().collect();

                        for id in keys {
                            self.sound.cleanup_entity_sounds(id);
                        }
                    }

                    // Cleanup 2d sounds
                    {
                        let sounds: Vec<SoundType> = self.sound.active_sounds.keys().cloned().collect();

                        for sound in sounds {
                            self.sound.stop_sound(&sound);
                        }
                    }

                    self.world = world;
                    self.physics = physics;
                }
                _ => {},
            }
        }

        let screen_size = glam::vec2(
            self.platform.fb_width as f32,
            self.platform.fb_height as f32,
        );

        self.engine_ui.update(
            &mut self.world.ecs,
            self.world.camera.move_state,
            &mut self.world.lights,
            &mut self.renderer,
            &mut self.sound,
            &mut self.input,
            &mut self.physics,
            &self.world.camera,
            screen_size,
            &mut self.world.particles,
            self.time.dt,
            &mut self.message_queue,
        );

        // Update game UI (pause menu) and handle actions
        self.game_ui.update(PauseMenuContext {
            paused: &mut self.paused,
            render_gizmos: &mut self.renderer.render_gizmos,
            message_queue: &mut self.message_queue,
            entity_manager: &self.world.ecs,
        });
    }

    pub fn render(&mut self) {
        self.renderer.draw(
            &self.world.ecs,
            &mut self.world.camera,
            &self.world.lights,
            &mut self.sound,
            self.platform.fb_width,
            self.platform.fb_height,
            self.time.elapsed,
            &self.physics,
            self.time.alpha,
            &mut self.world.particles,
        );

        self.imgui_manager.draw(
            &mut self.platform.window, 
            self.platform.fb_width as f32, 
            self.platform.fb_height as f32, 
            self.time.dt, 
            &mut self.world.lights, 
            &mut self.renderer, 
            &mut self.sound, 
            &self.world.camera, 
            &mut self.world.ecs,
            &mut self.physics,
            &mut self.input,
            &mut self.world.particles,
            &mut self.message_queue,
        );

        self.engine_ui.render(
            self.renderer.shaders.get_mut(&ShaderType::UiOverlay).unwrap(),
            &self.world.camera,
        );

        if self.paused {
            self.game_ui.render(self.renderer.shaders.get_mut(&ShaderType::UiOverlay).unwrap());
        }

        self.platform
            .surface
            .swap_buffers(&self.platform.gl_context)
            .expect("swap_buffers failed");
    }
}
