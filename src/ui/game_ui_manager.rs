//! Manages the Slint-based game UI as an overlay on top of the OpenGL scene.
//! This is separate from EngineUiManager which handles the engine/editor UI.
//!
//! Uses a single GameRoot Slint component that contains both the pause menu
//! and the player HUD, rendered via software rendering to a GL texture.

use std::rc::Rc;

use slint::platform::software_renderer::{MinimalSoftwareWindow, PremultipliedRgbaColor};
use slint::platform::PointerEventButton;
use slint::platform::WindowEvent as SlintWindowEvent;
use slint::{LogicalPosition, PhysicalSize};
use winit::event::WindowEvent;

use crate::entity_manager::EntityManager;
use crate::gl_call;
use crate::input::InputState;
use crate::lights::Lights;
use crate::renderer::{DefaultTextures, Renderer, UiTextureDescriptor, UiUploadBuffer};
use crate::shaders::Shader;
use crate::ui::game::views::ability_bar::AbilityBarData;
use crate::ui::game::views::{GameRootContext, GameRootView, SettingsContext, SystemContext};
use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::views::ability_bar_view::AbilityBarView;
use crate::ui::game_new::views::game_hud::{GameHudView, PlayerHudData};
use crate::ui::game_new::views::pause_menu_view::{PauseMenuUpdateContext, PauseMenuView};
use crate::ui::game_new::views::toast_view::ToastView;
use crate::ui::image_cache::UiImageCache;
use crate::ui::message_queue::MessageQueue;
use crate::ui::portrait_renderer::PortraitRenderer;
use crate::ui::slint_platform::init_slint_platform;

/// Context for GameUiManager::update() - contains all state that UI views may need.
pub struct GameUiUpdateContext<'a> {
    pub message_queue: &'a mut MessageQueue,
    pub entity_manager: &'a EntityManager,
    pub paused: &'a mut bool,
    pub render_gizmos: &'a mut bool, // renderer.render_gizmos - kept separate since it's not in config
    pub game_config: &'a mut crate::config::game_config::GameConfig,
    pub sound_config: &'a mut crate::config::sound_config::SoundConfig,
    pub elapsed_time: f64,
    pub input_state: &'a crate::input::InputState,
}

/// Context for portrait rendering - passed to render().
pub struct PortraitRenderContext<'a> {
    pub entity_manager: &'a EntityManager,
    pub shader: &'a mut Shader,
    pub lights: &'a Lights,
    pub defaults: &'a DefaultTextures,
    pub cubemap: u32,
    pub elapsed_time: f64,
}

/// Manages the Slint-based game UI as an overlay on top of the OpenGL scene.
/// Uses a single window with software rendering to a pixel buffer, uploaded to a GL texture.
pub struct GameUiManager {
    window: Rc<MinimalSoftwareWindow>,
    buffer: Vec<PremultipliedRgbaColor>,
    texture: u32,
    game_root_view: GameRootView,
    portrait_renderer: PortraitRenderer,
    ability_bar_view: AbilityBarView,
    image_cache: UiImageCache,

    width: u32,
    height: u32,
    /// current window scale factor (winit's physical / logical)
    scale_factor: f32,
    last_cursor_pos: LogicalPosition,
    needs_texture_resize: bool,
    overlay_vao: u32,
    overlay_vbo: u32,
    ui_upload_buffer: UiUploadBuffer,

    is_paused: bool,

    game_hud_view: GameHudView,
    pause_menu_view: PauseMenuView,
    toast_view: ToastView,
    ui_renderer: UiRenderer,
    font_system: FontSystem,

    // throttling for main UI update and render (pause menu + HUD)
    last_update_time: f64,
    last_render_time: f64,

    // scroll event accumulation (to throttle scroll event dispatching)
    accumulated_scroll_x: f32,
    accumulated_scroll_y: f32,

    // current font family for GPU UI rendering
    current_font_family: Option<String>,
}

impl GameUiManager {
    /// Create a new GameUiManager and initialize the Slint platform for game UI.
    pub fn new(width: u32, height: u32, scale_factor: f32) -> Self {
        // initialize the Slint platform before creating any Slint components
        init_slint_platform(width, height, scale_factor);

        // create unified game root view
        let (game_root_view, window) = GameRootView::new(width, height, scale_factor);
        let pixel_count = (width * height) as usize;
        let buffer = vec![PremultipliedRgbaColor::default(); pixel_count];
        let texture = Renderer::create_ui_texture(
            UiTextureDescriptor::rgba_linear_clamped(width, height),
            None,
        );

        // create VAO/VBO for fullscreen quad overlay
        let (overlay_vao, overlay_vbo) = Self::create_overlay_quad();

        // Native builds currently back this with a PBO; WebGL can use the same handle-shaped API differently.
        let ui_upload_buffer = Renderer::create_ui_upload_buffer();

        // create portrait renderer for player HUD
        let portrait_renderer = PortraitRenderer::new();

        // create font system and UI renderer first (needed for HUD and ability bar)
        let mut font_system = FontSystem::new();
        let mut ui_renderer = UiRenderer::new();
        ui_renderer.set_screen_size(width as f32, height as f32);

        // create ability bar view (new custom UI system)
        let mut ability_bar_view = AbilityBarView::new(&mut font_system);
        ability_bar_view.set_screen_size(width as f32, height as f32);

        // create game HUD view
        let mut game_hud_view = GameHudView::new(&mut font_system);
        game_hud_view
            .tree
            .set_screen_size(width as f32, height as f32);

        let mut pause_menu_view = PauseMenuView::new(&mut font_system);
        pause_menu_view.set_screen_size(width as f32, height as f32);

        let mut toast_view = ToastView::new(&mut font_system);
        toast_view.set_screen_size(width as f32, height as f32);

        let manager = Self {
            window,
            buffer,
            texture,
            game_root_view,
            portrait_renderer,
            ability_bar_view,
            image_cache: UiImageCache::new(),
            width,
            height,
            scale_factor,
            last_cursor_pos: LogicalPosition::new(0.0, 0.0),
            needs_texture_resize: false,
            overlay_vao,
            overlay_vbo,
            ui_upload_buffer,
            is_paused: false,
            game_hud_view,
            pause_menu_view,
            toast_view,
            ui_renderer,
            font_system,
            last_update_time: -999.0, // force first update
            last_render_time: -999.0, // force first render
            accumulated_scroll_x: 0.0,
            accumulated_scroll_y: 0.0,
            current_font_family: None,
        };

        manager
    }

    /// Create VAO/VBO for fullscreen quad overlay
    fn create_overlay_quad() -> (u32, u32) {
        unsafe {
            let mut vao = 0u32;
            let mut vbo = 0u32;
            gl_call!(gl::GenVertexArrays(1, &mut vao));
            gl_call!(gl::GenBuffers(1, &mut vbo));

            let vertices: [f32; 24] = [
                -1.0, 1.0, 0.0, 0.0, -1.0, -1.0, 0.0, 1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 0.0,
                0.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0,
            ];

            gl_call!(gl::BindVertexArray(vao));
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, vbo));
            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * std::mem::size_of::<f32>()) as isize,
                vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            ));
            gl_call!(gl::EnableVertexAttribArray(0));
            gl_call!(gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                16,
                std::ptr::null()
            ));
            gl_call!(gl::EnableVertexAttribArray(1));
            gl_call!(gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                16,
                8 as *const _
            ));
            gl_call!(gl::BindVertexArray(0));
            (vao, vbo)
        }
    }
    /// Handle a winit window event. Only processes events when paused (for pause menu).
    pub fn handle_window_event(&mut self, event: &WindowEvent, _input: &mut InputState) -> bool {
        let slint_event = match event {
            WindowEvent::CursorMoved { position, .. } => {
                let logical_x = (position.x as f32) / self.scale_factor.max(0.0001);
                let logical_y = (position.y as f32) / self.scale_factor.max(0.0001);
                self.last_cursor_pos = LogicalPosition::new(logical_x, logical_y);
                Some(SlintWindowEvent::PointerMoved {
                    position: self.last_cursor_pos,
                })
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let btn = match button {
                    winit::event::MouseButton::Left => PointerEventButton::Left,
                    winit::event::MouseButton::Right => PointerEventButton::Right,
                    winit::event::MouseButton::Middle => PointerEventButton::Middle,
                    _ => return false,
                };
                match state {
                    winit::event::ElementState::Pressed => Some(SlintWindowEvent::PointerPressed {
                        position: self.last_cursor_pos,
                        button: btn,
                    }),
                    winit::event::ElementState::Released => {
                        Some(SlintWindowEvent::PointerReleased {
                            position: self.last_cursor_pos,
                            button: btn,
                        })
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                use winit::event::MouseScrollDelta;
                let (delta_x, delta_y) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        // convert lines to pixels (roughly 20px per line)
                        (*x * 20.0, *y * 20.0)
                    }
                    MouseScrollDelta::PixelDelta(p) => (p.x as f32, p.y as f32),
                };
                // accumulate scroll deltas instead of immediately dispatching
                // (will be dispatched in update() at 60 Hz to prevent Slint from doing expensive work 100+ times/sec)
                self.accumulated_scroll_x += delta_x;
                self.accumulated_scroll_y += delta_y;
                None // don't dispatch immediately
            }
            WindowEvent::Resized(size) => {
                self.resize(size.width, size.height);
                None
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = *scale_factor as f32;
                Some(SlintWindowEvent::ScaleFactorChanged {
                    scale_factor: self.scale_factor,
                })
            }
            _ => None,
        };

        if let Some(ev) = slint_event {
            self.window.dispatch_event(ev);
            true
        } else {
            false
        }
    }

    /// Resize the UI to match the new window size.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;
        let pixel_count = (width * height) as usize;
        self.buffer
            .resize(pixel_count, PremultipliedRgbaColor::default());
        self.window.set_size(PhysicalSize::new(width, height));
        self.needs_texture_resize = true;

        self.ui_renderer
            .set_screen_size(width as f32, height as f32);
        self.game_hud_view
            .tree
            .set_screen_size(width as f32, height as f32);

        // update ability bar screen size for positioning
        self.ability_bar_view
            .set_screen_size(width as f32, height as f32);

        // update pause menu screen size
        self.pause_menu_view
            .set_screen_size(width as f32, height as f32);

        // update toast view screen size
        self.toast_view.set_screen_size(width as f32, height as f32);
    }

    /// Update the UI each frame.
    pub fn update(&mut self, ctx: GameUiUpdateContext) {
        // throttle Slint updates to 60 Hz
        const UPDATE_INTERVAL: f64 = 1.0 / 60.0;
        if ctx.elapsed_time - self.last_update_time >= UPDATE_INTERVAL {
            slint::platform::update_timers_and_animations();
            self.last_update_time = ctx.elapsed_time;
        }

        // dispatch accumulated scroll events (throttled to 60 Hz to prevent Slint from doing expensive work)
        if self.accumulated_scroll_x != 0.0 || self.accumulated_scroll_y != 0.0 {
            self.window
                .dispatch_event(SlintWindowEvent::PointerScrolled {
                    position: self.last_cursor_pos,
                    delta_x: self.accumulated_scroll_x,
                    delta_y: self.accumulated_scroll_y,
                });
            self.accumulated_scroll_x = 0.0;
            self.accumulated_scroll_y = 0.0;
        }

        // Sync our local paused state with the engine state
        self.is_paused = *ctx.paused;

        if !self.is_paused {
            let game_ctx = GameRootContext {
                paused: ctx.paused,
                settings: SettingsContext {
                    render_gizmos: ctx.render_gizmos,
                    game_config: ctx.game_config,
                    sound_config: ctx.sound_config,
                },
                system: SystemContext {
                    entity_manager: ctx.entity_manager,
                    message_queue: ctx.message_queue,
                },
                image_cache: &mut self.image_cache,
                elapsed_time: ctx.elapsed_time,
            };
            self.game_root_view.update(game_ctx);
        }

        // Update new Game HUD
        let hud_data = PlayerHudData::from_entity_manager(ctx.entity_manager);
        let portrait_tex = self.portrait_renderer.get_texture_id();

        let mut ui_ctx = UiContext {
            input: ctx.input_state,
            messages: ctx.message_queue,
        };

        self.game_hud_view
            .update(&mut ui_ctx, &hud_data, portrait_tex);

        // update ability bar view (new custom UI system)
        let ability_data = AbilityBarData::from_entity_manager(ctx.entity_manager);
        let delta_time = (ctx.elapsed_time - self.last_update_time) as f32;

        // Convert AbilityBarData individual fields to array for new view
        let slots = [
            ability_data.q.clone(),
            ability_data.e.clone(),
            ability_data.r.clone(),
        ];

        self.ability_bar_view.update_data(&slots, delta_time.abs());

        let mut ui_ctx2 = UiContext {
            input: ctx.input_state,
            messages: ctx.message_queue,
        };
        self.ability_bar_view.update(&mut ui_ctx2);

        // drain pending toasts from global queue and add them to toast view
        let pending_toasts = crate::ui::toast::drain_pending_toasts();
        if !pending_toasts.is_empty() {
            println!(
                "[GameUiManager] Draining {} pending toasts",
                pending_toasts.len()
            );
        }
        for toast in pending_toasts {
            self.toast_view
                .add_toast(toast.toast_type, toast.title, toast.message, toast.duration);
        }

        // update toast view time and state
        // Use the actual elapsed time from the game, not a delta
        self.toast_view.set_elapsed_time(ctx.elapsed_time);
        let mut ui_ctx3 = UiContext {
            input: ctx.input_state,
            messages: ctx.message_queue,
        };
        self.toast_view.update(&mut ui_ctx3);

        self.current_font_family = Some(ctx.game_config.font_family.clone());

        if self.is_paused {
            let mut pause_ctx = PauseMenuUpdateContext {
                paused: ctx.paused,
                render_gizmos: ctx.render_gizmos,
                game_config: ctx.game_config,
                sound_config: ctx.sound_config,
                entity_manager: ctx.entity_manager,
                message_queue: ctx.message_queue,
                input_state: ctx.input_state,
            };
            self.pause_menu_view
                .update(&mut pause_ctx, &mut self.font_system);
        }
    }

    /// Set the current FPS for the FPS counter.
    pub fn set_fps(&self, fps: i32) {
        self.game_root_view.set_fps(fps);
    }

    /// Render the player portrait to the HUD. Call this before render().
    /// Uses throttling to avoid rendering every frame (30 Hz is plenty for a small portrait).
    pub fn render_portrait(&mut self, ctx: PortraitRenderContext) {
        // only render if enough time has passed (throttled to 30 Hz)
        if !self.portrait_renderer.should_update(ctx.elapsed_time) {
            return;
        }

        if let Some(player_id) = ctx.entity_manager.get_player_id() {
            self.portrait_renderer.render_portrait(
                ctx.entity_manager,
                player_id,
                ctx.shader,
                ctx.lights,
                ctx.defaults,
                ctx.cubemap,
                ctx.elapsed_time,
            );
        }
    }

    pub fn render(&mut self, shader: &mut Shader, elapsed_time: f64) {
        // throttle main UI render to 60 Hz to avoid re-rendering pause menu at 700 Hz during scrolling
        const RENDER_INTERVAL: f64 = 1.0 / 60.0; // 60 Hz = ~16.6ms
        let should_render = elapsed_time - self.last_render_time >= RENDER_INTERVAL;

        // CRITICAL: Always trigger layout if needed, even if render is throttled
        // This ensures toasts are laid out immediately when added, not on the next render cycle
        if self.toast_view.needs_layout {
            println!("[GameUiManager::render] Toast needs layout, triggering layout now");
            self.toast_view.layout(&mut self.font_system);
        }

        if !should_render {
            // still draw the cached texture, just don't re-render Slint
            self.draw_cached_ui(shader);
            return;
        }

        self.last_render_time = elapsed_time;

        // --------------------------------------------------------
        // PART 1: UI texture upload
        // --------------------------------------------------------
        let texture_desc = UiTextureDescriptor::rgba_linear_clamped(self.width, self.height);
        if self.needs_texture_resize {
            Renderer::resize_ui_texture(self.texture, texture_desc);
        }

        // always call request_redraw - Slint's internal dirty tracking handles optimization
        // (attempting to throttle this causes crashes due to Slint's internal state management)
        self.window.request_redraw();

        self.window.draw_if_needed(|renderer| {
            let size = (self.width * self.height * 4) as isize;
            let pixel_count = (self.width * self.height) as usize;
            Renderer::write_ui_upload_buffer(
                self.ui_upload_buffer,
                size,
                pixel_count,
                self.needs_texture_resize,
                |buffer_slice: &mut [PremultipliedRgbaColor]| {
                    renderer.render(buffer_slice, self.width as usize);
                },
            );
            if self.needs_texture_resize {
                self.needs_texture_resize = false;
            }
        });

        Renderer::update_ui_texture_from_upload_buffer(
            self.texture,
            self.ui_upload_buffer,
            texture_desc,
        );

        // --------------------------------------------------------
        // PART 2: Compositing
        // --------------------------------------------------------
        unsafe {
            gl_call!(gl::Enable(gl::BLEND));
            gl_call!(gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA));
            gl_call!(gl::Disable(gl::DEPTH_TEST));
        }

        // A. DRAW SLINT UI (only when NOT paused - pause menu is now GPU-rendered)
        if !self.is_paused {
            self.draw_overlay(shader, self.texture);
        }

        // B. DRAW Custom GPU UI
        if !self.is_paused {
            // When not paused, draw ability bar and HUD
            if self.ability_bar_view.needs_layout() {
                self.ability_bar_view.layout(&mut self.font_system);
                self.ability_bar_view.clear_layout_flag();
            }

            if self.game_hud_view.needs_render() {
                self.game_hud_view.tree.layout(&mut self.font_system);
                self.game_hud_view.clear_render_flag();
            }

            // set current font for GPU UI rendering
            self.ui_renderer
                .set_default_font_family(self.current_font_family.clone());

            self.ui_renderer.begin();
            self.game_hud_view.tree.render(&mut self.ui_renderer);
            self.ability_bar_view.render(&mut self.ui_renderer);

            // render toast view LAST (overlay priority)
            self.toast_view.render(&mut self.ui_renderer);

            self.ui_renderer.end(&mut self.font_system);
        } else {
            // When paused, draw ONLY the GPU pause menu (no Slint overlay)
            if self.pause_menu_view.needs_layout() {
                self.pause_menu_view.tree.layout(&mut self.font_system);
                self.pause_menu_view.clear_layout_flag();
            }

            // set current font for GPU UI rendering
            self.ui_renderer
                .set_default_font_family(self.current_font_family.clone());

            self.ui_renderer.begin();
            self.pause_menu_view.tree.render(&mut self.ui_renderer);

            // render toast view LAST (overlay priority) - toasts persist when paused
            self.toast_view.render(&mut self.ui_renderer);

            self.ui_renderer.end(&mut self.font_system);
        }

        unsafe {
            gl_call!(gl::Enable(gl::DEPTH_TEST));
            gl_call!(gl::Disable(gl::BLEND));
        }
    }

    /// Draw the cached UI textures without re-rendering Slint.
    /// Used when throttling to avoid re-rendering at 700 Hz.
    fn draw_cached_ui(&mut self, shader: &mut Shader) {
        unsafe {
            gl_call!(gl::Enable(gl::BLEND));
            gl_call!(gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA));
            gl_call!(gl::Disable(gl::DEPTH_TEST));
        }

        // A. DRAW SLINT UI (only when NOT paused)
        if !self.is_paused {
            self.draw_overlay(shader, self.texture);
        }

        // B. DRAW Custom GPU UI
        if !self.is_paused {
            // set current font for GPU UI rendering
            self.ui_renderer
                .set_default_font_family(self.current_font_family.clone());

            self.ui_renderer.begin();
            self.game_hud_view.tree.render(&mut self.ui_renderer);
            self.ability_bar_view.render(&mut self.ui_renderer);

            // render toast view LAST (overlay priority)
            self.toast_view.render(&mut self.ui_renderer);

            self.ui_renderer.end(&mut self.font_system);
        } else {
            // When paused, draw pause menu + toasts
            // ! IMPORTANT: Check needs_layout even in cached render path
            // ! to ensure layout is updated after tab selection
            if self.pause_menu_view.needs_layout() {
                self.pause_menu_view.tree.layout(&mut self.font_system);
                self.pause_menu_view.clear_layout_flag();
            }

            // ! set current font for GPU UI rendering
            self.ui_renderer
                .set_default_font_family(self.current_font_family.clone());

            self.ui_renderer.begin();
            self.pause_menu_view.tree.render(&mut self.ui_renderer);

            // render toast view LAST (overlay priority) - toasts persist when paused
            self.toast_view.render(&mut self.ui_renderer);

            self.ui_renderer.end(&mut self.font_system);
        }

        unsafe {
            gl_call!(gl::Enable(gl::DEPTH_TEST));
            gl_call!(gl::Disable(gl::BLEND));
        }
    }

    fn draw_overlay(&self, shader: &crate::shaders::Shader, texture: u32) {
        unsafe {
            shader.activate();
            gl_call!(gl::ActiveTexture(gl::TEXTURE0));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, texture));
            shader.set_int("ui_texture", 0);

            // reset transform to fill the screen, no flip for Slint UI
            shader.set_vec2("u_offset", 0.0, 0.0);
            shader.set_vec2("u_scale", 1.0, 1.0);
            shader.set_bool("u_flip_v", false);

            gl_call!(gl::BindVertexArray(self.overlay_vao));
            gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 6));
            gl_call!(gl::BindVertexArray(0));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
        }
    }
}

impl Drop for GameUiManager {
    fn drop(&mut self) {
        unsafe {
            gl_call!(gl::DeleteTextures(1, &self.texture));
            gl_call!(gl::DeleteVertexArrays(1, &self.overlay_vao));
            gl_call!(gl::DeleteBuffers(1, &self.overlay_vbo));
            Renderer::delete_ui_upload_buffer(self.ui_upload_buffer);
        }
    }
}
