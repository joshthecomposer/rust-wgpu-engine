use std::path::Path;

use glutin::surface::{GlSurface, SwapInterval};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::Fullscreen;

use crate::animation::animation_system;
use crate::command_buffer::CommandBuffer;
use crate::config::game_config::GameConfig;
use crate::config::sound_config::SoundConfig;
use crate::config::Config;
use crate::enums_types::{CameraState, ShaderType, SoundType};
use crate::input::{self, InputState};
use crate::physics::PhysicsState;
use crate::platform::{CursorMode, Platform};
use crate::renderer::Renderer;
use crate::sound::sound_manager::SoundManager;
use crate::state_machines::state_machine_system;
use crate::time::Time;
use crate::toast;
// use crate::ui::game_new::parser::load_view_or_fallback;
// use crate::ui::game_new::{FontSystem, UiContext, UiRenderer, UiTree};
// use crate::ui::game_ui_manager::{GameUiManager, GameUiUpdateContext, PortraitRenderContext};
use crate::ui::imgui::imgui_manager::ImguiManager;
use crate::ui::message_queue::{MessageQueue, UiMessage};
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
    pub paused: bool,
    cursor_mode: CursorMode,
    message_queue: MessageQueue,
    // game_ui: GameUiManager,
    // custom_ui: Option<UiTree>,
    // gallery_ui: Option<UiTree>,
    // custom_ui_renderer: UiRenderer,
    // font_system: FontSystem,
    pub should_quit: bool,
    imgui_manager: Option<ImguiManager>,
    config: GameConfig,
    config_path: String,
    sound_config: SoundConfig,
    sound_config_path: String,
    command_buffer: CommandBuffer,
}

impl Game {
    pub fn cursor_unlocked(&self) -> bool {
        self.cursor_mode == CursorMode::Normal
    }

    pub fn new(platform: Platform, config: GameConfig) -> Self {
        let start_seconds = 0.0;
        let time = Time::new(60.0, start_seconds);

        let mut physics = PhysicsState::new();
        let mut world = World::new();

        world.ecs.populate_entity_data(&mut physics);

        let sound_config = SoundConfig::load_or_create_default("config/sound_config.json");

        let renderer = Renderer::new(&platform, &config);
        let sound = SoundManager::new(&sound_config);
        // let game_ui = GameUiManager::new(
        //     platform.fb_width,
        //     platform.fb_height,
        //     platform.scale_factor as f32,
        // );

        let imgui_manager = match config.debug_mode {
            true => Some(ImguiManager::new(&platform)),
            false => None,
        };

        // let custom_ui = None;

        // let mut gallery_ui = load_view_or_fallback("resources/ui/gallery_view.ron");
        // gallery_ui.set_screen_size(platform.fb_width as f32, platform.fb_height as f32);
        // let gallery_ui = Some(gallery_ui);

        // let mut custom_ui_renderer = UiRenderer::new();
        // let font_system = FontSystem::new();
        // custom_ui_renderer.set_screen_size(platform.fb_width as f32, platform.fb_height as f32);

        Self {
            platform,
            time,
            physics,
            world,
            renderer,
            sound,
            input: InputState::new(),
            paused: false,
            cursor_mode: CursorMode::Hidden,
            message_queue: MessageQueue::new(),
            // game_ui,
            // custom_ui,
            // gallery_ui,
            // custom_ui_renderer,
            // font_system,
            should_quit: false,
            imgui_manager,
            config,
            config_path: "config/game_config.json".to_string(),
            sound_config,
            sound_config_path: "config/sound_config.json".to_string(),
            command_buffer: CommandBuffer::default(),
        }
    }

    /// Check if any settings changed that require an application restart.
    /// Returns true if restart is required, false otherwise.
    fn check_settings_require_restart(&self, old_config: &GameConfig) -> bool {
        let msaa_changed = old_config.msaa_level != self.config.msaa_level;
        let debug_mode_changed = old_config.debug_mode != self.config.debug_mode;

        // log individual changes for debugging
        if msaa_changed {
            println!(
                "[DEBUG] MSAA changed: {} -> {}",
                old_config.msaa_level, self.config.msaa_level
            );
        }
        if debug_mode_changed {
            println!(
                "[DEBUG] Debug mode changed: {} -> {}",
                old_config.debug_mode, self.config.debug_mode
            );
        }

        msaa_changed || debug_mode_changed
    }

    pub fn tick(&mut self, now_seconds: f32) {
        self.time.begin_frame(now_seconds);

        // Mouse lock / cursor mode
        let hardware_mode = if self.paused || self.world.camera.move_state == CameraState::Locked {
            CursorMode::Normal
        } else {
            self.cursor_mode
        };
        self.platform.set_cursor_mode(hardware_mode);

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
                    &mut self.input,
                    &mut self.command_buffer,
                    self.time.fixed_dt,
                );

                self.world.spawn_manager.update(
                    &mut self.world.ecs,
                    &mut self.physics,
                    self.time.fixed_dt,
                    self.config.spawn_system_enabled,
                );

                items::update(&mut self.world.ecs, &mut self.physics);

                animation_system::update(
                    &mut self.world.ecs,
                    &mut self.command_buffer,
                    self.time.fixed_dt,
                );

                combat_system::update(&mut self.world.ecs, self.time.fixed_dt, &mut self.physics);

                self.world.ecs.update(
                    &mut self.sound,
                    &mut self.physics,
                    &mut self.input,
                    self.time.fixed_dt,
                );

                physics::push_static_kinematics(&self.world.ecs, &mut self.physics);

                match self.world.camera.move_state {
                    CameraState::Third | CameraState::Locked => {
                        movement_system::update(
                            &mut self.world.ecs,
                            &cam_basis,
                            &mut self.command_buffer,
                            &mut self.physics,
                            self.time.fixed_dt,
                        );
                    }
                    _ => {}
                }

                self.physics
                    .evaluate_commands(&mut self.world.ecs, &mut self.command_buffer);

                self.physics.step();

                physics::sync_transforms_from_physics(&mut self.world.ecs, &self.physics);
                physics::sync_collider_transforms_with_physics(
                    &mut self.world.ecs,
                    &mut self.physics,
                );
                physics::push_weapon_kinematics_from_bones(&mut self.world.ecs, &mut self.physics);
            }

            self.time.end_fixed_step();
        }

        self.time.end_frame();

        self.update();
        self.render();

        if stepped {
            self.input.update();
        }

        self.input.update_ui();
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        //self.game_ui.handle_window_event(event, &mut self.input);
        if let Some(imgui_manager) = &mut self.imgui_manager {
            imgui_manager.handle_imgui_event(event);
        }
        match event {
            WindowEvent::Resized(size) => {
                self.platform.fb_width = size.width;
                self.platform.fb_height = size.height;
                // if let Some(tree) = &mut self.custom_ui {
                //     tree.set_screen_size(size.width as f32, size.height as f32);
                // }
                // if let Some(tree) = &mut self.gallery_ui {
                //     tree.set_screen_size(size.width as f32, size.height as f32);
                // }
                // self.custom_ui_renderer
                //     .set_screen_size(size.width as f32, size.height as f32);
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                // keep platform scale factor in sync and update framebuffer size from window
                self.platform.scale_factor = *scale_factor;
                let size = self.platform.window.inner_size();
                self.platform.fb_width = size.width;
                self.platform.fb_height = size.height;
                // resize Slint game UI to match new framebuffer size
                //self.game_ui.resize(size.width, size.height);
            }

            WindowEvent::DroppedFile(path) => {
                if let Some(imgui_manager) = &mut self.imgui_manager {
                    let path = Path::new(path);
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        match ext {
                            "txt" => {
                                imgui_manager.entity_editor.new_archetype.mesh_path =
                                    path.to_string_lossy().into_owned();
                            }
                            "png" | "jpg" | "jpeg" => {
                                imgui_manager.entity_editor.new_archetype.texture_path =
                                    path.to_string_lossy().into_owned();
                                imgui_manager.particle_editor.staged_texture =
                                    path.to_string_lossy().into_owned();
                                imgui_manager.ability_editor.staged_icon =
                                    path.to_string_lossy().into_owned();
                            }
                            _ => {}
                        }
                    }
                }
            }

            WindowEvent::CloseRequested => {}

            WindowEvent::MouseWheel { delta, .. } => {
                // Capture scroll wheel delta for UI
                use winit::event::MouseScrollDelta;
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(x, y) => glam::vec2(*x, *y),
                    MouseScrollDelta::PixelDelta(pos) => glam::vec2(pos.x as f32, pos.y as f32),
                };
                self.input.scroll_delta = scroll;
            }

            WindowEvent::CursorMoved { position, .. } => {
                if !self.paused {
                    self.world.camera.process_mouse_input_movement(*position);
                }
                self.input.mouse_pos_current = glam::vec2(position.x as f32, position.y as f32);
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(imgui_manager) = &mut self.imgui_manager {
                    let io = imgui_manager.imgui.io();
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
                } else {
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

                    if let Some(imgui_manager) = &mut self.imgui_manager {
                        let io = imgui_manager.imgui.io();
                        // ignore if imgui has keyboard focus
                        if !io.want_capture_keyboard {
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
                                    CameraState::Locked => CameraState::Gallery,
                                    CameraState::Gallery => CameraState::Free,
                                };
                            }

                            if keycode == KeyCode::KeyG && *state == ElementState::Pressed {
                                self.world.ecs.try_pickup_weapon(&mut self.physics);
                            }
                        }
                    } else {
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
                                CameraState::Locked => CameraState::Gallery,
                                CameraState::Gallery => CameraState::Free,
                            };
                        }

                        if keycode == KeyCode::KeyG && *state == ElementState::Pressed {
                            self.world.ecs.try_pickup_weapon(&mut self.physics);
                        }
                    }

                    // Tab toggles cursor unlock (for hovering over abilities)
                    if keycode == KeyCode::Tab && *state == ElementState::Pressed {
                        self.cursor_mode = match self.cursor_mode {
                            CursorMode::Normal => CursorMode::Hidden,
                            _ => CursorMode::Normal,
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
        self.sound
            .update(&self.world.camera, &mut self.command_buffer);
        self.world.lights.update(&self.time.dt);
        self.world
            .particles
            .update(self.time.dt, &mut self.command_buffer, &self.world.ecs);

        // update custom GPU UI
        // if self.world.camera.move_state == CameraState::Gallery {
        //     if let Some(tree) = &mut self.gallery_ui {
        //         tree.layout(&mut self.font_system);
        //         let mut ctx = UiContext {
        //             input: &self.input,
        //             messages: &mut self.message_queue,
        //         };
        //         if tree.update(&mut ctx) {
        //             // Widget state changed (e.g., scrolling), re-layout to apply changes
        //             tree.force_layout();
        //             tree.layout(&mut self.font_system);
        //         }
        //     }
        // } else {
        //     if let Some(tree) = &mut self.custom_ui {
        //         tree.layout(&mut self.font_system);
        //         let mut ctx = UiContext {
        //             input: &self.input,
        //             messages: &mut self.message_queue,
        //         };
        //         if tree.update(&mut ctx) {
        //             // Widget state changed, re-layout
        //             tree.force_layout();
        //             tree.layout(&mut self.font_system);
        //         }
        //     }
        // }

        // update game UI (pause menu, HUD, etc.) BEFORE processing messages
        // this ensures UI values are synced to game state before ApplySettings saves
        // self.game_ui.update(GameUiUpdateContext {
        //     message_queue: &mut self.message_queue,
        //     entity_manager: &self.world.ecs,
        //     paused: &mut self.paused,
        //     render_gizmos: &mut self.renderer.render_gizmos,
        //     game_config: &mut self.config,
        //     sound_config: &mut self.sound_config,
        //     elapsed_time: self.time.elapsed as f64,
        //     input_state: &self.input,
        // });

        // self.game_ui.set_fps(self.time.fps);

        let msgs = self.message_queue.drain();

        for msg in msgs.iter() {
            match msg {
                UiMessage::WindowShouldClose => {
                    self.should_quit = true;
                }
                UiMessage::RenderStagedEmitters { do_it } => {
                    self.world.particles.render_staged_emitters = *do_it;
                }
                UiMessage::ReloadWorldData => {
                    let mut world = World::new();
                    let mut physics = PhysicsState::new();

                    world.ecs.populate_entity_data(&mut physics);

                    // Cleanup 3d sounds
                    {
                        let keys: Vec<usize> =
                            self.sound.active_3d_sounds.keys().cloned().collect();

                        for id in keys {
                            self.sound.cleanup_entity_sounds(id);
                        }
                    }

                    // Cleanup 2d sounds
                    {
                        let sounds: Vec<SoundType> =
                            self.sound.active_sounds.keys().cloned().collect();

                        for sound in sounds {
                            self.sound.stop_sound(&sound);
                        }
                    }

                    self.world = world;
                    self.physics = physics;

                    toast!(
                        Success,
                        "World Reloaded",
                        "World data has been reloaded successfully."
                    );
                }
                UiMessage::ApplySettings => {
                    // load old config to detect changes that require restart
                    let old_config = GameConfig::load_from_file(&self.config_path);
                    let requires_restart = self.check_settings_require_restart(&old_config);

                    // sync renderer state to config before saving
                    self.config.render_gizmos = self.renderer.render_gizmos;

                    // apply resolution if it changed
                    let current_size = self.platform.window.inner_size();
                    let new_width = self.config.win_width as u32;
                    let new_height = self.config.win_height as u32;

                    if current_size.width != new_width || current_size.height != new_height {
                        let _ = self
                            .platform
                            .window
                            .request_inner_size(LogicalSize::new(new_width, new_height));
                        println!(
                            "[DEBUG] ApplySettings - Requested window resize to {}x{}",
                            new_width, new_height
                        );
                    }

                    // apply vsync setting
                    if self.config.vsync {
                        let _ = self.platform.surface.set_swap_interval(
                            &self.platform.gl_context,
                            SwapInterval::Wait(std::num::NonZeroU32::new(1).unwrap()),
                        );
                        println!("[DEBUG] ApplySettings - VSync enabled");
                    } else {
                        let _ = self
                            .platform
                            .surface
                            .set_swap_interval(&self.platform.gl_context, SwapInterval::DontWait);
                        println!("[DEBUG] ApplySettings - VSync disabled");
                    }

                    // apply window mode setting
                    match self.config.window_mode.as_str() {
                        "Windowed" => {
                            self.platform.window.set_fullscreen(None);
                            println!("[DEBUG] ApplySettings - Window mode set to Windowed");
                        }
                        "Fullscreen" => {
                            // get current monitor and its video mode
                            if let Some(monitor) = self.platform.window.current_monitor() {
                                if let Some(video_mode) = monitor.video_modes().next() {
                                    self.platform
                                        .window
                                        .set_fullscreen(Some(Fullscreen::Exclusive(video_mode)));
                                    println!(
                                        "[DEBUG] ApplySettings - Window mode set to Fullscreen"
                                    );
                                }
                            }
                        }
                        "Borderless" => {
                            self.platform
                                .window
                                .set_fullscreen(Some(Fullscreen::Borderless(None)));
                            println!("[DEBUG] ApplySettings - Window mode set to Borderless");
                        }
                        _ => {
                            println!(
                                "[WARN] ApplySettings - Unknown window mode: {}",
                                self.config.window_mode
                            );
                        }
                    }

                    // save configs to disk
                    self.config.save_to_file(&self.config_path);
                    self.sound_config.save_to_file(&self.sound_config_path);

                    self.paused = false;

                    println!("[DEBUG] ApplySettings - Configs saved to disk");

                    // show appropriate toast based on whether restart is required
                    if requires_restart {
                        toast!(
                            Info,
                            "Restart Required",
                            "Some changes will take effect after restarting the application."
                        );
                    } else {
                        toast!(
                            Success,
                            "Settings Applied",
                            "Your settings have been saved successfully."
                        );
                    }
                }
                UiMessage::CancelSettings => {
                    // reload configs from disk to discard changes
                    self.config = GameConfig::load_from_file(&self.config_path);
                    self.sound_config = SoundConfig::load_from_file(&self.sound_config_path);

                    // sync renderer state from reloaded config
                    self.renderer.render_gizmos = self.config.render_gizmos;

                    toast!(
                        Info,
                        "Settings Cancelled",
                        "Your changes have been discarded."
                    );
                }
            }
        }
    }

    pub fn render(&mut self) {
        self.renderer.draw(
            &mut self.world.ecs,
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

        // render player portrait for HUD (uses animated model shader)
        {
            // let anim_shader = self
            //     .renderer
            //     .shaders
            //     .get_mut(&ShaderType::AnimatedModel)
            //     .unwrap();
            // let portrait_ctx = PortraitRenderContext {
            //     entity_manager: &self.world.ecs,
            //     shader: anim_shader,
            //     lights: &self.world.lights,
            //     defaults: &self.renderer.defaults,
            //     cubemap: self.renderer.cubemap_texture,
            //     elapsed_time: self.time.elapsed as f64,
            // };
            // self.game_ui.render_portrait(portrait_ctx);
        }

        // render game UI overlay (pause menu when paused, HUD when playing)
        // let ui_shader = self
        //     .renderer
        //     .shaders
        //     .get_mut(&ShaderType::UiOverlay)
        //     .unwrap();
        // self.game_ui.render(ui_shader, self.time.elapsed as f64);

        // render custom GPU UI (test view)
        // if self.world.camera.move_state == CameraState::Gallery {
        //     if let Some(tree) = &self.gallery_ui {
        //         self.custom_ui_renderer.begin();
        //         tree.render(&mut self.custom_ui_renderer);
        //         self.custom_ui_renderer.end(&mut self.font_system);
        //     }
        // } else {
        //     if let Some(tree) = &self.custom_ui {
        //         self.custom_ui_renderer.begin();
        //         tree.render(&mut self.custom_ui_renderer);
        //         self.custom_ui_renderer.end(&mut self.font_system);
        //     }
        // }

        if let Some(imgui_manager) = &mut self.imgui_manager {
            imgui_manager.draw(
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
        }

        self.platform
            .surface
            .swap_buffers(&self.platform.gl_context)
            .expect("swap_buffers failed");
    }
}
