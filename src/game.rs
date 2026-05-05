#[cfg(all(feature = "editor_ui", not(target_arch = "wasm32")))]
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use glutin::surface::{GlSurface, SwapInterval};
#[cfg(not(target_arch = "wasm32"))]
use winit::dpi::LogicalSize;
#[cfg(not(target_arch = "wasm32"))]
use winit::event::{ElementState, KeyEvent, WindowEvent};
#[cfg(not(target_arch = "wasm32"))]
use winit::keyboard::{KeyCode, PhysicalKey};
#[cfg(not(target_arch = "wasm32"))]
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
use crate::shaders::ShaderProfile;
use crate::sound::sound_manager::SoundManager;
use crate::state_machines::state_machine_system;
use crate::time::Time;
use crate::ui::game_new::parser::load_view_or_fallback;
use crate::ui::game_new::views::game_hud::{GameHudView, PlayerHudData};
use crate::ui::game_new::views::pause_menu_view::{PauseMenuUpdateContext, PauseMenuView};
use crate::ui::game_new::{FontSystem, UiContext, UiRenderer, UiTree};
#[cfg(all(feature = "editor_ui", not(target_arch = "wasm32")))]
use crate::ui::imgui::imgui_manager::ImguiManager;
use crate::ui::message_queue::{MessageQueue, UiMessage};
use crate::ui::portrait_renderer::PortraitRenderer;
use crate::world::World;
use crate::{combat_system, items, movement_system, physics};
use crate::{projectile_system, toast};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

const UI_ENABLED: bool = true;

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
    pause_menu: PauseMenuView,
    game_hud: GameHudView,
    portrait_renderer: PortraitRenderer,
    gallery_ui: Option<UiTree>,
    custom_ui_renderer: UiRenderer,
    font_system: FontSystem,
    pub should_quit: bool,
    #[cfg(all(feature = "editor_ui", not(target_arch = "wasm32")))]
    imgui_manager: Option<ImguiManager>,
    config: GameConfig,
    config_path: String,
    sound_config: SoundConfig,
    sound_config_path: String,
    command_buffer: CommandBuffer,
}

impl Game {
    fn show_in_game_hud(&self) -> bool {
        UI_ENABLED
            && !self.paused
            && self.world.camera.move_state != CameraState::Gallery
            && self.world.ecs.get_player_id().is_some()
    }

    /// Portrait FBO refresh (throttled), then GPU HUD (vitals + ability bar).
    fn render_in_game_overlay(&mut self) {
        if !self.show_in_game_hud() {
            return;
        }

        if let Some(pid) = self.world.ecs.get_player_id() {
            if self
                .portrait_renderer
                .should_update(self.time.elapsed as f64)
            {
                let has_anim = self.world.ecs.animators.get(pid).is_some();
                let shader_ty = if has_anim {
                    ShaderType::AnimatedModel
                } else {
                    ShaderType::StaticModel
                };
                if let Some(shader) = self.renderer.shaders.get_mut(&shader_ty) {
                    self.portrait_renderer.render_portrait(
                        &self.world.ecs,
                        pid,
                        shader,
                        &self.world.lights,
                        &self.renderer.defaults,
                        self.renderer.cubemap_texture,
                        self.time.elapsed as f64,
                    );
                }
            }
        }

        self.custom_ui_renderer.begin();
        self.game_hud.tree.render(&mut self.custom_ui_renderer);
        self.custom_ui_renderer.end(&mut self.font_system);
    }

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

        let webgl_compatibility_mode =
            config.webgl_compatibility_mode || platform.capabilities.is_gles_like;

        #[cfg(all(feature = "editor_ui", not(target_arch = "wasm32")))]
        let imgui_manager = match UI_ENABLED && config.debug_mode && !webgl_compatibility_mode {
            true => Some(ImguiManager::new(&platform)),
            false => None,
        };

        let ui_shader_profile = if webgl_compatibility_mode {
            ShaderProfile::GlslEs300
        } else {
            ShaderProfile::DesktopCore
        };

        let mut custom_ui_renderer = UiRenderer::new_with_profile(ui_shader_profile);
        let mut font_system = FontSystem::new();

        let mut pause_menu = PauseMenuView::new(&mut font_system);
        pause_menu.set_screen_size(platform.fb_width as f32, platform.fb_height as f32);

        let mut game_hud = GameHudView::new(&mut font_system);
        game_hud.set_screen_size(platform.fb_width as f32, platform.fb_height as f32);

        let portrait_renderer = PortraitRenderer::new();

        let gallery_ui = if UI_ENABLED {
            let mut gallery_ui = load_view_or_fallback("resources/ui/gallery_view.ron");
            gallery_ui.set_screen_size(platform.fb_width as f32, platform.fb_height as f32);
            Some(gallery_ui)
        } else {
            None
        };

        custom_ui_renderer.set_screen_size(platform.fb_width as f32, platform.fb_height as f32);

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
            pause_menu,
            game_hud,
            portrait_renderer,
            gallery_ui,
            custom_ui_renderer,
            font_system,
            should_quit: false,
            #[cfg(all(feature = "editor_ui", not(target_arch = "wasm32")))]
            imgui_manager,
            config,
            config_path: "config/game_config.json".to_string(),
            sound_config,
            sound_config_path: "config/sound_config.json".to_string(),
            command_buffer: CommandBuffer::default(),
        }
    }

    fn check_settings_require_restart(&self, old_config: &GameConfig) -> bool {
        let msaa_changed = old_config.msaa_level != self.config.msaa_level;
        let debug_mode_changed = old_config.debug_mode != self.config.debug_mode;
        let compatibility_mode_changed =
            old_config.webgl_compatibility_mode != self.config.webgl_compatibility_mode;

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
        if compatibility_mode_changed {
            println!(
                "[DEBUG] WebGL compatibility mode changed: {} -> {}",
                old_config.webgl_compatibility_mode, self.config.webgl_compatibility_mode
            );
        }

        msaa_changed || debug_mode_changed || compatibility_mode_changed
    }

    pub fn tick(&mut self, now_seconds: f32) {
        #[cfg(target_arch = "wasm32")]
        {
            if self.platform.sync_canvas_buffer_to_display(1280, 720) {
                self.renderer.resize_webgl_compatibility_framebuffers(
                    &self.platform.capabilities,
                    self.platform.fb_width,
                    self.platform.fb_height,
                );
                self.custom_ui_renderer.set_screen_size(
                    self.platform.fb_width as f32,
                    self.platform.fb_height as f32,
                );
                if let Some(ref mut gallery) = self.gallery_ui {
                    gallery.set_screen_size(
                        self.platform.fb_width as f32,
                        self.platform.fb_height as f32,
                    );
                }
                self.pause_menu.set_screen_size(
                    self.platform.fb_width as f32,
                    self.platform.fb_height as f32,
                );
                self.game_hud.set_screen_size(
                    self.platform.fb_width as f32,
                    self.platform.fb_height as f32,
                );
            }
        }

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

                projectile_system::update(
                    &mut self.world.ecs,
                    &mut self.command_buffer,
                    &mut self.physics,
                );

                combat_system::update(
                    &mut self.world.ecs,
                    self.time.fixed_dt,
                    &mut self.physics,
                    &mut self.command_buffer,
                );

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

    #[cfg(not(target_arch = "wasm32"))]
    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        #[cfg(feature = "editor_ui")]
        if UI_ENABLED {
            if let Some(imgui_manager) = &mut self.imgui_manager {
                imgui_manager.handle_imgui_event(event);
            }
        }
        match event {
            WindowEvent::Resized(size) => {
                self.platform.fb_width = size.width;
                self.platform.fb_height = size.height;
                if UI_ENABLED {
                    self.pause_menu
                        .set_screen_size(size.width as f32, size.height as f32);
                    self.game_hud
                        .set_screen_size(size.width as f32, size.height as f32);
                    if let Some(tree) = &mut self.gallery_ui {
                        tree.set_screen_size(size.width as f32, size.height as f32);
                    }
                    self.custom_ui_renderer
                        .set_screen_size(size.width as f32, size.height as f32);
                }
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                // keep platform scale factor in sync and update framebuffer size from window
                self.platform.scale_factor = *scale_factor;
                let size = self.platform.window.inner_size();
                self.platform.fb_width = size.width;
                self.platform.fb_height = size.height;
                if UI_ENABLED {
                    self.pause_menu
                        .set_screen_size(size.width as f32, size.height as f32);
                    self.game_hud
                        .set_screen_size(size.width as f32, size.height as f32);
                    if let Some(tree) = &mut self.gallery_ui {
                        tree.set_screen_size(size.width as f32, size.height as f32);
                    }
                    self.custom_ui_renderer
                        .set_screen_size(size.width as f32, size.height as f32);
                }
            }

            WindowEvent::DroppedFile(path) =>
            {
                #[cfg(feature = "editor_ui")]
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
                let mut mouse_captured_by_imgui = false;
                #[cfg(feature = "editor_ui")]
                if let Some(imgui_manager) = &mut self.imgui_manager {
                    let io = imgui_manager.imgui.io();
                    mouse_captured_by_imgui = io.want_capture_mouse;
                }

                if !mouse_captured_by_imgui {
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
                    let mut keyboard_captured_by_imgui = false;

                    #[cfg(feature = "editor_ui")]
                    if let Some(imgui_manager) = &mut self.imgui_manager {
                        let io = imgui_manager.imgui.io();
                        keyboard_captured_by_imgui = io.want_capture_keyboard;
                    }

                    if !keyboard_captured_by_imgui {
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

    #[cfg(target_arch = "wasm32")]
    pub fn handle_web_keyboard_input(
        &mut self,
        keycode: winit::keyboard::KeyCode,
        state: winit::event::ElementState,
    ) {
        input::handle_keyboard_input(keycode, state, &mut self.input);

        if keycode == winit::keyboard::KeyCode::Escape
            && state == winit::event::ElementState::Pressed
        {
            self.paused = !self.paused;
        }

        if keycode == winit::keyboard::KeyCode::KeyF && state == winit::event::ElementState::Pressed
        {
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

        if keycode == winit::keyboard::KeyCode::KeyG && state == winit::event::ElementState::Pressed
        {
            self.world.ecs.try_pickup_weapon(&mut self.physics);
        }

        if keycode == winit::keyboard::KeyCode::Tab && state == winit::event::ElementState::Pressed
        {
            self.cursor_mode = match self.cursor_mode {
                CursorMode::Normal => CursorMode::Hidden,
                _ => CursorMode::Normal,
            };
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn handle_web_mouse_move(&mut self, x: f32, y: f32, dx: f64, dy: f64) {
        let p = self.platform.canvas_css_to_framebuffer_px(x, y);
        self.input.mouse_pos_current = p;
        if !self.paused && !self.cursor_unlocked() {
            self.world.camera.process_mouse_input(dx, dy);
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn handle_web_mouse_button(
        &mut self,
        button: winit::event::MouseButton,
        state: winit::event::ElementState,
    ) {
        let fb = glam::vec2(
            self.platform.fb_width as f32,
            self.platform.fb_height as f32,
        );
        input::handle_mouse_input(
            button,
            state,
            fb,
            &self.world.camera,
            &mut self.world.ecs,
            &mut self.input,
            &mut self.physics,
        );
    }

    #[cfg(target_arch = "wasm32")]
    pub fn handle_web_scroll(&mut self, x: f32, y: f32) {
        self.input.scroll_delta = glam::vec2(x, y);
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

        if UI_ENABLED {
            // update custom GPU UI
            if self.world.camera.move_state == CameraState::Gallery {
                if let Some(tree) = &mut self.gallery_ui {
                    tree.layout(&mut self.font_system);
                    let mut ctx = UiContext {
                        input: &self.input,
                        messages: &mut self.message_queue,
                    };
                    if tree.update(&mut ctx) {
                        tree.force_layout();
                        tree.layout(&mut self.font_system);
                    }
                }
            } else if self.paused {
                let mut pause_ctx = PauseMenuUpdateContext {
                    paused: &mut self.paused,
                    render_gizmos: &mut self.renderer.render_gizmos,
                    game_config: &mut self.config,
                    sound_config: &mut self.sound_config,
                    entity_manager: &self.world.ecs,
                    message_queue: &mut self.message_queue,
                    input_state: &self.input,
                };
                self.pause_menu
                    .update(&mut pause_ctx, &mut self.font_system);
            } else if self.show_in_game_hud() {
                let data = PlayerHudData::from_entity_manager(&self.world.ecs);
                let portrait_tex = self.portrait_renderer.get_texture_id();
                let mut ui_ctx = UiContext {
                    input: &self.input,
                    messages: &mut self.message_queue,
                };
                self.game_hud.update(
                    &mut self.font_system,
                    &mut ui_ctx,
                    &data,
                    portrait_tex,
                    &self.world.ecs,
                    self.time.dt,
                );
            }
        }

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

                    self.apply_platform_settings();

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
                    #[cfg(target_arch = "wasm32")]
                    {
                        println!("[DEBUG] CancelSettings ignored on web");
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        // reload configs from disk to discard changes
                        self.config = GameConfig::load_from_file(&self.config_path);
                        self.sound_config = SoundConfig::load_from_file(&self.sound_config_path);
                    }

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

    #[cfg(not(target_arch = "wasm32"))]
    fn apply_platform_settings(&mut self) {
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
                        println!("[DEBUG] ApplySettings - Window mode set to Fullscreen");
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
    }

    #[cfg(target_arch = "wasm32")]
    fn apply_platform_settings(&mut self) {
        self.renderer.render_gizmos = self.config.render_gizmos;
        println!("[DEBUG] ApplySettings - runtime-only web settings applied");
    }

    pub fn render(&mut self) {
        if self.config.webgl_compatibility_mode || self.platform.capabilities.is_gles_like {
            self.renderer.render_world_webgl_compatibility(
                &mut self.world.ecs,
                &mut self.world.camera,
                &self.world.lights,
                self.platform.fb_width,
                self.platform.fb_height,
                self.time.elapsed,
                &self.physics,
                self.time.alpha,
                &mut self.world.particles,
                &mut self.sound,
            );

            if UI_ENABLED {
                if self.world.camera.move_state == CameraState::Gallery {
                    if let Some(tree) = &self.gallery_ui {
                        self.custom_ui_renderer.begin();
                        tree.render(&mut self.custom_ui_renderer);
                        self.custom_ui_renderer.end(&mut self.font_system);
                    }
                } else if self.paused {
                    self.custom_ui_renderer.begin();
                    self.pause_menu.tree.render(&mut self.custom_ui_renderer);
                    self.custom_ui_renderer.end(&mut self.font_system);
                } else {
                    self.render_in_game_overlay();
                }
            }

            self.platform.swap_buffers();
            return;
        }

        self.renderer.render_world(
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

        if UI_ENABLED {
            if self.world.camera.move_state == CameraState::Gallery {
                if let Some(tree) = &self.gallery_ui {
                    self.custom_ui_renderer.begin();
                    tree.render(&mut self.custom_ui_renderer);
                    self.custom_ui_renderer.end(&mut self.font_system);
                }
            } else if self.paused {
                self.custom_ui_renderer.begin();
                self.pause_menu.tree.render(&mut self.custom_ui_renderer);
                self.custom_ui_renderer.end(&mut self.font_system);
            } else {
                self.render_in_game_overlay();
            }
        }

        #[cfg(feature = "editor_ui")]
        if UI_ENABLED {
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
        }

        self.platform.swap_buffers();
    }
}
