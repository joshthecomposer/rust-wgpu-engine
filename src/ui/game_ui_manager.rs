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
use crate::renderer::DefaultTextures;
use crate::shaders::Shader;
use crate::ui::game::views::{GameRootContext, GameRootView};
use crate::ui::message_queue::MessageQueue;
use crate::ui::portrait_renderer::PortraitRenderer;
use crate::ui::slint_platform::init_slint_platform;

/// Context for GameUiManager::update() - contains all state that UI views may need.
pub struct GameUiUpdateContext<'a> {
    pub message_queue: &'a mut MessageQueue,
    pub entity_manager: &'a EntityManager,
    pub paused: &'a mut bool,
    pub render_gizmos: &'a mut bool,
}

/// Context for portrait rendering - passed to render().
pub struct PortraitRenderContext<'a> {
    pub entity_manager: &'a EntityManager,
    pub shader: &'a mut Shader,
    pub lights: &'a Lights,
    pub defaults: &'a DefaultTextures,
    pub cubemap: u32,
}

/// Manages the Slint-based game UI as an overlay on top of the OpenGL scene.
/// Uses a single window with software rendering to a pixel buffer, uploaded to a GL texture.
pub struct GameUiManager {
    window: Rc<MinimalSoftwareWindow>,
    buffer: Vec<PremultipliedRgbaColor>,
    texture: u32,
    game_root_view: GameRootView,
    portrait_renderer: PortraitRenderer,

    width: u32,
    height: u32,
    /// current window scale factor (winit's physical / logical)
    scale_factor: f32,
    last_cursor_pos: LogicalPosition,
    needs_texture_resize: bool,
    overlay_vao: u32,
    overlay_vbo: u32,
    pbo: u32,

    is_paused: bool,
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
        let texture = Self::create_gl_texture(width, height);

        // create VAO/VBO for fullscreen quad overlay
        let (overlay_vao, overlay_vbo) = Self::create_overlay_quad();

        // create PBO for async texture upload
        let mut pbo = 0;
        unsafe {
            gl_call!(gl::GenBuffers(1, &mut pbo));
        }

        // create portrait renderer for player HUD
        let portrait_renderer = PortraitRenderer::new();

        Self {
            window,
            buffer,
            texture,
            game_root_view,
            portrait_renderer,
            width,
            height,
            scale_factor,
            last_cursor_pos: LogicalPosition::new(0.0, 0.0),
            needs_texture_resize: false,
            overlay_vao,
            overlay_vbo,
            pbo,
            is_paused: false,
        }
    }

    /// Create a GL texture for UI overlay
    fn create_gl_texture(width: u32, height: u32) -> u32 {
        unsafe {
            let mut tex = 0u32;
            gl_call!(gl::GenTextures(1, &mut tex));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, tex));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                gl::LINEAR as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAG_FILTER,
                gl::LINEAR as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_EDGE as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_EDGE as i32
            ));
            gl_call!(gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as i32,
                width as i32,
                height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                std::ptr::null()
            ));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
            tex
        }
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
    }

    /// Update the UI each frame.
    pub fn update(&mut self, ctx: GameUiUpdateContext) {
        slint::platform::update_timers_and_animations();

        // Sync our local paused state with the engine state
        self.is_paused = *ctx.paused;

        let game_ctx = GameRootContext {
            paused: ctx.paused,
            render_gizmos: ctx.render_gizmos,
            entity_manager: ctx.entity_manager,
            message_queue: ctx.message_queue,
        };
        self.game_root_view.update(game_ctx);
    }

    /// Render the player portrait to the HUD. Call this before render().
    pub fn render_portrait(&mut self, ctx: PortraitRenderContext) {
        if let Some(player_id) = ctx.entity_manager.get_player_id() {
            self.portrait_renderer.render_portrait(
                ctx.entity_manager,
                player_id,
                ctx.shader,
                ctx.lights,
                ctx.defaults,
                ctx.cubemap,
            );
        }
    }

    pub fn render(&mut self, shader: &mut Shader) {
        // --------------------------------------------------------
        // PART 1: PBO Upload (Zero-Copy Slint Render)
        // --------------------------------------------------------
        if self.needs_texture_resize {
            Self::resize_texture(self.texture, self.width, self.height);
        }

        self.window.request_redraw();

        self.window.draw_if_needed(|renderer| unsafe {
            gl_call!(gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, self.pbo));

            if self.needs_texture_resize {
                let size = (self.width * self.height * 4) as isize;
                gl_call!(gl::BufferData(
                    gl::PIXEL_UNPACK_BUFFER,
                    size,
                    std::ptr::null(),
                    gl::STREAM_DRAW
                ));
                self.needs_texture_resize = false;
            }

            let ptr = gl::MapBuffer(gl::PIXEL_UNPACK_BUFFER, gl::WRITE_ONLY);
            if !ptr.is_null() {
                let pixel_count = (self.width * self.height) as usize;
                let buffer_slice = std::slice::from_raw_parts_mut(
                    ptr as *mut slint::platform::software_renderer::PremultipliedRgbaColor,
                    pixel_count,
                );
                renderer.render(buffer_slice, self.width as usize);
                gl_call!(gl::UnmapBuffer(gl::PIXEL_UNPACK_BUFFER));
            }
        });

        unsafe {
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, self.texture));
            gl_call!(gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                self.width as i32,
                self.height as i32,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                std::ptr::null()
            ));
            gl_call!(gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, 0));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
        }

        // --------------------------------------------------------
        // PART 2: Compositing
        // --------------------------------------------------------
        unsafe {
            gl_call!(gl::Enable(gl::BLEND));
            gl_call!(gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA));
            gl_call!(gl::Disable(gl::DEPTH_TEST));
        }

        // A. DRAW UI (Background Layer)
        self.draw_overlay(shader, self.texture);

        // B. DRAW PORTRAIT (Foreground Layer - inside the transparent frame)
        // Only draw if NOT paused!
        if !self.is_paused {
            let portrait_tex = self.portrait_renderer.get_texture_id();
            // portrait is rendered to FBO so needs V-flip
            // the portrait FBO is square, so we need equal NDC dimensions
            // adjusted for window aspect ratio to appear square on screen
            // Slint frame is ~56x56 pixels, window is typically wider than tall
            let aspect = self.width as f32 / self.height as f32;
            let size = 0.08; // base size in NDC (height)
            let width = size / aspect; // adjust width for aspect ratio
            self.draw_screen_quad(
                shader,
                portrait_tex,
                -0.92, // x position in NDC
                0.86,  // y position in NDC
                width, // width adjusted for aspect ratio
                size,  // height in NDC
                true,  // flip_v for FBO texture
            );
        }

        unsafe {
            gl_call!(gl::Enable(gl::DEPTH_TEST));
            gl_call!(gl::Disable(gl::BLEND));
        }
    }

    /// Helper to draw a quad at a specific position/scale
    fn draw_screen_quad(
        &self,
        shader: &mut Shader,
        texture: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        flip_v: bool,
    ) {
        unsafe {
            shader.activate();

            gl_call!(gl::ActiveTexture(gl::TEXTURE0));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, texture));
            shader.set_int("ui_texture", 0);

            shader.set_vec2("u_offset", x, y);
            shader.set_vec2("u_scale", w, h);
            shader.set_bool("u_flip_v", flip_v);

            gl_call!(gl::BindVertexArray(self.overlay_vao));
            gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 6));
            gl_call!(gl::BindVertexArray(0));
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

    /// Resize a GL texture
    fn resize_texture(texture: u32, width: u32, height: u32) {
        unsafe {
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, texture));
            gl_call!(gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as i32,
                width as i32,
                height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                std::ptr::null(),
            ));
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
            gl_call!(gl::DeleteBuffers(1, &self.pbo));
        }
    }
}
