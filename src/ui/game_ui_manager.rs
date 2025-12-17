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
                std::ptr::null(),
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
                // pos (x, y)    uv (u, v)
                -1.0, 1.0, 0.0, 0.0, -1.0, -1.0, 0.0, 1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 0.0, 0.0,
                1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0,
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
                // keep our local scale factor in sync with winit and forward to Slint
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

    /// Resize the UI.
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
            let pixels = self.portrait_renderer.render_portrait(
                ctx.entity_manager,
                player_id,
                ctx.shader,
                ctx.lights,
                ctx.defaults,
                ctx.cubemap,
            );

            let portrait_image = Self::create_slint_image(pixels);
            self.game_root_view.set_player_portrait(portrait_image);
        }
    }

    /// Render the UI to the internal pixel buffer and upload to GL texture.
    pub fn render(&mut self, shader: &mut Shader) {
        if self.needs_texture_resize {
            Self::resize_texture(self.texture, self.width, self.height);
            self.needs_texture_resize = false;
        }

        self.window.request_redraw();
        self.window.draw_if_needed(|renderer| {
            renderer.render(&mut self.buffer, self.width as usize);
        });
        Self::upload_to_texture(self.texture, &self.buffer, self.width, self.height);

        self.draw_overlay(shader, self.texture);
    }

    /// Create a Slint Image from raw RGBA pixel data.
    fn create_slint_image(pixels: &[u8]) -> slint::Image {
        use crate::ui::portrait_renderer::PORTRAIT_SIZE;
        use slint::{Rgba8Pixel, SharedPixelBuffer};

        let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(PORTRAIT_SIZE, PORTRAIT_SIZE);
        buffer.make_mut_bytes().copy_from_slice(pixels);
        slint::Image::from_rgba8(buffer)
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

    /// Upload pixel buffer to GL texture
    fn upload_to_texture(texture: u32, buffer: &[PremultipliedRgbaColor], width: u32, height: u32) {
        unsafe {
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, texture));
            gl_call!(gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                width as i32,
                height as i32,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                buffer.as_ptr() as *const _,
            ));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
        }
    }

    /// Draw a UI overlay on screen.
    fn draw_overlay(&self, shader: &crate::shaders::Shader, texture: u32) {
        unsafe {
            gl_call!(gl::Enable(gl::BLEND));
            gl_call!(gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA));
            gl_call!(gl::Disable(gl::DEPTH_TEST));

            shader.activate();
            gl_call!(gl::ActiveTexture(gl::TEXTURE0));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, texture));
            shader.set_int("ui_texture", 0);

            gl_call!(gl::BindVertexArray(self.overlay_vao));
            gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 6));
            gl_call!(gl::BindVertexArray(0));

            gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
            gl_call!(gl::Enable(gl::DEPTH_TEST));
            gl_call!(gl::Disable(gl::BLEND));
        }
    }
}

impl Drop for GameUiManager {
    fn drop(&mut self) {
        unsafe {
            gl_call!(gl::DeleteTextures(1, &self.texture));
            gl_call!(gl::DeleteVertexArrays(1, &self.overlay_vao));
            gl_call!(gl::DeleteBuffers(1, &self.overlay_vbo));
        }
    }
}
