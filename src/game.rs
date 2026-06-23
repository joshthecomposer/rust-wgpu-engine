#[cfg(all(feature = "editor_ui", not(target_arch = "wasm32")))]
use std::path::Path;
use std::sync::Arc;

use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::animation::animation_system;
use crate::camera::CameraUniform;
use crate::command_buffer::CommandBuffer;
use crate::config::game_config::GameConfig;
use crate::config::sound_config::SoundConfig;
use crate::config::Config;
use crate::enums_types::{CameraState, CursorMode, SoundType};
use crate::input::{self, InputState};
use crate::lights::DirLightUniform;
use crate::physics::PhysicsState;
use crate::platform::Platform;
use crate::sound::sound_manager::SoundManager;
use crate::state_machines::state_machine_system;
use crate::time::Time;
//use crate::ui::game_new::parser::load_view_or_fallback;
use crate::ui::game_new::views::game_hud::{GameHudView, PlayerHudData};
use crate::ui::game_new::views::pause_menu_view::{PauseMenuUpdateContext, PauseMenuView};
use crate::ui::game_new::{FontSystem, UiContext, UiRenderer};
#[cfg(all(feature = "editor_ui", not(target_arch = "wasm32")))]
use crate::ui::imgui::imgui_manager::ImguiManager;
use crate::ui::message_queue::{MessageQueue, UiMessage};
use crate::wgpu_backend::render_context::RenderContext;
//use crate::ui::portrait_renderer::PortraitRenderer;
use crate::wgpu_backend::renderer::Renderer;
use crate::world::World;
use crate::{
    damage_resolution_system, damage_volume_spawn_system, items, movement_system, physics,
    status_effect_system,
};
use crate::{projectile_system, toast};

const UI_ENABLED: bool = true;

pub struct Game {
    time: Time, // delta time, alpha time, elapsed time
    pub platform: Platform,
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
    //portrait_renderer: PortraitRenderer,
    //gallery_ui: Option<UiTree>,
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
    pub async fn new(platform: Platform, config: GameConfig) -> Self {
        let start_seconds = 0.0;
        let time = Time::new(60.0, start_seconds);

        let mut physics = PhysicsState::new();

        // Native runtime creates the winit `Window` first, then passes it via `platform.window`.
        let window = Arc::clone(
            platform
                .window
                .as_ref()
                .expect("missing window in Platform"),
        );
        let mut renderer = Renderer::new(
            window,
            CameraUniform::new(),
            DirLightUniform::new(),
        )
        .await;
        renderer.render_gizmos = config.render_gizmos;

        let rdr_ctx = RenderContext {
            device: &renderer.device,
            queue: &renderer.queue,
            layout: &renderer.shared_layouts.texture,
        };

        let mut world = World::new(&rdr_ctx);
        world.ecs.populate_entity_data(&mut physics, &rdr_ctx);

        let sound_config = SoundConfig::load_or_create_default("config/sound_config.json");

        let sound = SoundManager::new(&sound_config);

        #[cfg(all(feature = "editor_ui", not(target_arch = "wasm32")))]
        let imgui_manager =
            match UI_ENABLED && config.debug_mode && !config.webgl_compatibility_mode {
                true => Some(ImguiManager::new(
                    &renderer.device,
                    &renderer.queue,
                    renderer.surface_view_format,
                )),
                false => None,
            };

        // Custom RON UI.
        // TODO: portrait renderer aren't ported yyet
        let mut custom_ui_renderer =
            UiRenderer::new(&renderer.device, &renderer.queue, renderer.surface_view_format);
        custom_ui_renderer.set_screen_size(platform.fb_width as f32, platform.fb_height as f32);

        let mut font_system = FontSystem::new();

        let mut pause_menu = PauseMenuView::new(&mut font_system);
        pause_menu.set_screen_size(platform.fb_width as f32, platform.fb_height as f32);

        let mut game_hud = GameHudView::new(&mut font_system);
        game_hud.set_screen_size(platform.fb_width as f32, platform.fb_height as f32);

        //let portrait_renderer = PortraitRenderer::new();

        //let gallery_ui = if UI_ENABLED {
        //    let mut gallery_ui = load_view_or_fallback("resources/ui/gallery_view.ron");
        //    gallery_ui.set_screen_size(platform.fb_width as f32, platform.fb_height as f32);
        //    Some(gallery_ui)
        //} else {
        //    None
        //};

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
            //portrait_renderer,
            //gallery_ui,
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
    fn show_in_game_hud(&self) -> bool {
        UI_ENABLED
            && !self.paused
            && self.world.camera.move_state != CameraState::Gallery
            && self.world.ecs.get_player_id().is_some()
    }

    /// Portrait FBO refresh (throttled), then GPU HUD (vitals + ability bar).
    fn render_in_game_overlay(&mut self) {
        //if !self.show_in_game_hud() {
        //    return;
        //}

        //if let Some(pid) = self.world.ecs.get_player_id() {
        //    if self
        //        .portrait_renderer
        //        .should_update(self.time.elapsed as f64)
        //    {
        //        let has_anim = self.world.ecs.animators.get(pid).is_some();
        //        let shader_ty = if has_anim {
        //            ShaderType::AnimatedModel
        //        } else {
        //            ShaderType::StaticModel
        //        };
        //        if let Some(shader) = self.renderer.shaders.get_mut(&shader_ty) {
        //            self.portrait_renderer.render_portrait(
        //                &self.world.ecs,
        //                pid,
        //                shader,
        //                &self.world.lights,
        //                &self.renderer.defaults,
        //                self.renderer.cubemap_texture,
        //                self.time.elapsed as f64,
        //            );
        //        }
        //    }
        //}

        //self.custom_ui_renderer.begin();
        //self.game_hud.tree.render(&mut self.custom_ui_renderer);
        //self.custom_ui_renderer.end(&mut self.font_system);
    }

    //pub fn cursor_unlocked(&self) -> bool {
    //    self.cursor_mode == CursorMode::Normal
    //}

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

    pub fn resize(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        self.platform.fb_width = width;
        self.platform.fb_height = height;
        self.renderer.resize(width, height);
        self.custom_ui_renderer
            .set_screen_size(width as f32, height as f32);
        self.pause_menu.set_screen_size(width as f32, height as f32);
        self.game_hud.set_screen_size(width as f32, height as f32);
    }

    pub fn tick(&mut self, now_seconds: f32) {
        self.time.begin_frame(now_seconds);

        // Web: winit doesn't deliver reliable `Resized` events for a CSS-sized
        // canvas, so poll its size here and resize (surface + camera aspect + UI)
        // when it changes.
        #[cfg(target_arch = "wasm32")]
        if let Some(window) = self.platform.window.clone() {
            let (cw, ch) = crate::platform::web_canvas_physical_size(&window);
            if cw != self.platform.fb_width || ch != self.platform.fb_height {
                self.resize(cw, ch);
            }
        }

        // Mouse lock / cursor mode
        let hardware_mode = if self.paused || self.world.camera.move_state == CameraState::Locked {
            CursorMode::Normal
        } else {
            self.cursor_mode
        };

        self.platform.set_winit_cursor_mode(hardware_mode);

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
                    self.time.fixed_dt,
                );

                damage_volume_spawn_system::update(&mut self.world.ecs, &mut self.physics);

                physics::push_damage_volume_kinematics(&mut self.world.ecs, &mut self.physics);

                damage_resolution_system::update(
                    &mut self.world.ecs,
                    self.time.fixed_dt,
                    &mut self.physics,
                    &mut self.command_buffer,
                );

                status_effect_system::update(
                    &mut self.world.ecs,
                    self.time.fixed_dt,
                    &mut self.command_buffer,
                );

                self.world.ecs.update(
                    &mut self.sound,
                    &mut self.physics,
                    &mut self.input,
                    self.time.fixed_dt,
                    &mut self.command_buffer,
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
        #[cfg(all(feature = "editor_ui", not(target_arch = "wasm32")))]
        if UI_ENABLED {
            if let Some(imgui_manager) = &mut self.imgui_manager {
                imgui_manager.handle_imgui_event(event);
            }
        }
        match event {
            WindowEvent::DroppedFile(path) =>
            {
                #[cfg(all(feature = "editor_ui", not(target_arch = "wasm32")))]
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
                #[cfg(all(feature = "editor_ui", not(target_arch = "wasm32")))]
                let mouse_captured_by_imgui = self
                    .imgui_manager
                    .as_ref()
                    .is_some_and(|imgui_manager| imgui_manager.imgui.io().want_capture_mouse);
                #[cfg(not(all(feature = "editor_ui", not(target_arch = "wasm32"))))]
                let mouse_captured_by_imgui = false;

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

                    #[cfg(all(feature = "editor_ui", not(target_arch = "wasm32")))]
                    let keyboard_captured_by_imgui =
                        self.imgui_manager.as_ref().is_some_and(|imgui_manager| {
                            imgui_manager.imgui.io().want_capture_keyboard
                        });
                    #[cfg(not(all(feature = "editor_ui", not(target_arch = "wasm32"))))]
                    let keyboard_captured_by_imgui = false;

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

    pub fn update(&mut self) {
        self.world.camera.update(
            &self.world.ecs,
            self.time.dt,
            &self.physics,
            self.time.alpha,
            &self.input,
            self.platform.fb_width as f32 / self.platform.fb_height as f32,
        );
        self.sound.update(
            &self.world.camera,
            &mut self.command_buffer,
            &mut self.world.ecs,
            &self.physics,
            self.time.dt,
        );
        self.world.lights.update(&self.time.dt);
        self.world
            .particles
            .update(self.time.dt, &mut self.command_buffer, &self.world.ecs);

        if UI_ENABLED {
            // Custom GPU UI. The gallery branch stays commented out until
            // the gallery FBO / portrait FBO are ported off OpenGL.
            //
            //if self.world.camera.move_state == CameraState::Gallery {
            //    if let Some(tree) = &mut self.gallery_ui { ... }
            //} else
            if self.paused {
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
                // Portrait FBO isn't ported yet; pass `0` so the HUD's
                // TextureRect renders the fallback (1x1 white) until the
                // portrait renderer registers a real texture id.
                let portrait_tex: u32 = 0;
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
                    let rdr = RenderContext {
                        device: &self.renderer.device,
                        queue: &self.renderer.queue,
                        layout: &self.renderer.shared_layouts.texture,
                    };

                    let mut world = World::new(&rdr);
                    let mut physics = PhysicsState::new();

                    world.ecs.populate_entity_data(&mut physics, &rdr);

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
                    //// load old config to detect changes that require restart
                    //let old_config = GameConfig::load_from_file(&self.config_path);
                    //let requires_restart = self.check_settings_require_restart(&old_config);

                    //// sync renderer state to config before saving
                    //self.config.render_gizmos = self.renderer.render_gizmos;

                    //self.paused = false;

                    //println!("[DEBUG] ApplySettings - Configs saved to disk");

                    //// show appropriate toast based on whether restart is required
                    //if requires_restart {
                    //    toast!(
                    //        Info,
                    //        "Restart Required",
                    //        "Some changes will take effect after restarting the application."
                    //    );
                    //} else {
                    //    toast!(
                    //        Success,
                    //        "Settings Applied",
                    //        "Your settings have been saved successfully."
                    //    );
                    //}
                }
                UiMessage::CancelSettings => {
                    //#[cfg(target_arch = "wasm32")]
                    //{
                    //    println!("[DEBUG] CancelSettings ignored on web");
                    //}
                    //#[cfg(not(target_arch = "wasm32"))]
                    //{
                    //    // reload configs from disk to discard changes
                    //    self.config = GameConfig::load_from_file(&self.config_path);
                    //    self.sound_config = SoundConfig::load_from_file(&self.sound_config_path);
                    //}

                    //// sync renderer state from reloaded config
                    //self.renderer.render_gizmos = self.config.render_gizmos;

                    //toast!(
                    //    Info,
                    //    "Settings Cancelled",
                    //    "Your changes have been discarded."
                    //);
                }
            }
        }
    }

    pub fn render(&mut self) {
        self.platform.window.as_ref().unwrap().request_redraw();

        self.custom_ui_renderer.begin();
        if UI_ENABLED {
            if self.paused {
                self.pause_menu.tree.render(&mut self.custom_ui_renderer);
            } else if self.show_in_game_hud() {
                self.game_hud.tree.render(&mut self.custom_ui_renderer);
            }
        }
        self.custom_ui_renderer.end(&mut self.font_system);
        self.custom_ui_renderer
            .prepare(&self.renderer.device, &self.renderer.queue);

        #[cfg(all(feature = "editor_ui", not(target_arch = "wasm32")))]
        let prepared_imgui = if let Some(imgui_manager) = &mut self.imgui_manager {
            imgui_manager.prepare_render(
                self.platform.fb_width as f32,
                self.platform.fb_height as f32,
                self.time.dt,
                &mut self.world.lights,
                &mut self.sound,
                &self.world.camera,
                &mut self.world.ecs,
                &mut self.physics,
                &mut self.input,
                &mut self.world.particles,
                &mut self.message_queue,
                &self.renderer.device,
                &self.renderer.queue,
            )
        } else {
            None
        };

        let custom_ui = &self.custom_ui_renderer;

        #[cfg(all(feature = "editor_ui", not(target_arch = "wasm32")))]
        if let Some(prepared_imgui) = prepared_imgui {
            self.renderer.render_world_with_overlay(
                &self.world.camera,
                &self.world.ecs,
                self.time.alpha,
                &self.world.lights,
                &mut self.world.particles,
                Some(
                    |_device: &wgpu::Device,
                     _queue: &wgpu::Queue,
                     encoder: &mut wgpu::CommandEncoder,
                     surface_view: &wgpu::TextureView| {
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("ui overlay pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: surface_view,
                                resolve_target: None,
                                depth_slice: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            occlusion_query_set: None,
                            timestamp_writes: None,
                            multiview_mask: None,
                        });

                        // Custom RON UI first, then imgui on top of it.
                        custom_ui.render(&mut rpass);

                        if let Err(err) = prepared_imgui.render(&mut rpass) {
                            eprintln!("imgui render failed: {err}");
                        }
                    },
                ),
            );
            return;
        }

        self.renderer.render_world_with_overlay(
            &self.world.camera,
            &self.world.ecs,
            self.time.alpha,
            &self.world.lights,
            &mut self.world.particles,
            Some(
                |_device: &wgpu::Device,
                 _queue: &wgpu::Queue,
                 encoder: &mut wgpu::CommandEncoder,
                 surface_view: &wgpu::TextureView| {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("ui overlay pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: surface_view,
                            resolve_target: None,
                            depth_slice: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                        multiview_mask: None,
                    });

                    custom_ui.render(&mut rpass);
                },
            ),
        );
    }
}
