//! This is separate from EngineUiManager which handles the engine/editor UI.

use std::rc::Rc;

use slint::platform::software_renderer::{MinimalSoftwareWindow, PremultipliedRgbaColor};
use slint::platform::PointerEventButton;
use slint::platform::WindowEvent as SlintWindowEvent;
use slint::{LogicalPosition, PhysicalSize};
use winit::event::WindowEvent;

use crate::entity_manager::EntityManager;
use crate::gl_call;
use crate::input::InputState;
use crate::shaders::Shader;
use crate::ui::game::views::{PauseMenuContext, PauseMenuView};
use crate::ui::message_queue::MessageQueue;

/// View-specific data for the pause menu.
pub struct PauseMenuData<'a> {
    pub paused: &'a mut bool,
    pub render_gizmos: &'a mut bool,
}

/// Context for GameUiManager::update() - contains all state that UI views may need.
/// This is the public API that game.rs uses; view-specific contexts are internal.
pub struct GameUiUpdateContext<'a> {
    // shared across all views
    pub message_queue: &'a mut MessageQueue,
    pub entity_manager: &'a EntityManager,

    // view-specific data (grouped)
    pub pause_menu: PauseMenuData<'a>,
}

/// Manages the Slint-based game UI as an overlay on top of the OpenGL scene.
/// Uses software rendering to a pixel buffer, which is then uploaded to a GL texture.
pub struct GameUiManager {
    window: Rc<MinimalSoftwareWindow>,
    pixel_buffer: Vec<PremultipliedRgbaColor>,
    width: u32,
    height: u32,
    last_cursor_pos: LogicalPosition,
    gl_texture: u32,
    needs_texture_resize: bool,
    overlay_vao: u32,
    overlay_vbo: u32,
    // UI views
    pause_menu_view: PauseMenuView,
}

impl GameUiManager {
    /// Create a new GameUiManager. Must be called AFTER EngineUiManager (platform must be initialized).
    pub fn new(width: u32, height: u32) -> Self {
        let (pause_menu_view, window) = PauseMenuView::new(width, height);

        let pixel_count = (width * height) as usize;
        let pixel_buffer = vec![PremultipliedRgbaColor::default(); pixel_count];

        // create GL texture
        let gl_texture = unsafe {
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
        };

        // create VAO/VBO for fullscreen quad overlay
        let (overlay_vao, overlay_vbo) = unsafe {
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
        };

        Self {
            window,
            pixel_buffer,
            width,
            height,
            last_cursor_pos: LogicalPosition::new(0.0, 0.0),
            gl_texture,
            needs_texture_resize: false,
            overlay_vao,
            overlay_vbo,
            pause_menu_view,
        }
    }

    /// Handle a winit window event. Only processes events when paused.
    pub fn handle_window_event(&mut self, event: &WindowEvent, _input: &mut InputState) -> bool {
        let slint_event = match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.last_cursor_pos = LogicalPosition::new(position.x as f32, position.y as f32);
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
        self.pixel_buffer
            .resize(pixel_count, PremultipliedRgbaColor::default());

        self.window.set_size(PhysicalSize::new(width, height));
        self.needs_texture_resize = true;
    }

    /// Update the UI each frame. Delegates to individual view components.
    /// Views handle their own logic and directly modify state via context refs.
    pub fn update(&mut self, ctx: GameUiUpdateContext) {
        slint::platform::update_timers_and_animations();

        let pause_ctx = PauseMenuContext {
            paused: ctx.pause_menu.paused,
            render_gizmos: ctx.pause_menu.render_gizmos,
            entity_manager: ctx.entity_manager,
            message_queue: ctx.message_queue,
        };
        self.pause_menu_view.update(pause_ctx);
    }

    /// Render the UI to the internal pixel buffer and upload to GL texture.
    pub fn render(&mut self, shader: &mut Shader) {
        if self.needs_texture_resize {
            unsafe {
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, self.gl_texture));
                gl_call!(gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as i32,
                    self.width as i32,
                    self.height as i32,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    std::ptr::null(),
                ));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
            }
            self.needs_texture_resize = false;
        }

        self.window.request_redraw();
        self.window.draw_if_needed(|renderer| {
            renderer.render(&mut self.pixel_buffer, self.width as usize);
        });

        // Upload to GL texture
        unsafe {
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, self.gl_texture));
            gl_call!(gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                self.width as i32,
                self.height as i32,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                self.pixel_buffer.as_ptr() as *const _,
            ));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
        }

        self.draw_overlay(shader);
    }

    /// Draw the UI overlay on screen.
    pub fn draw_overlay(&self, shader: &crate::shaders::Shader) {
        unsafe {
            gl_call!(gl::Enable(gl::BLEND));
            gl_call!(gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA));
            gl_call!(gl::Disable(gl::DEPTH_TEST));

            shader.activate();
            gl_call!(gl::ActiveTexture(gl::TEXTURE0));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, self.gl_texture));
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
            gl_call!(gl::DeleteTextures(1, &self.gl_texture));
            gl_call!(gl::DeleteVertexArrays(1, &self.overlay_vao));
            gl_call!(gl::DeleteBuffers(1, &self.overlay_vbo));
        }
    }
}
