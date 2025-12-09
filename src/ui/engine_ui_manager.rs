use std::rc::Rc;

use slint::platform::software_renderer::{MinimalSoftwareWindow, PremultipliedRgbaColor};
use slint::platform::PointerEventButton;
use slint::platform::WindowEvent as SlintWindowEvent;
use slint::{LogicalPosition, PhysicalSize};
use winit::event::WindowEvent;

use crate::ui::slint_platform::init_slint_platform;

slint::slint! {
    export component MainWindow inherits Window {
        background: transparent;

        Text {
            text: "Hello from Slint!";
            font-size: 24px;
            color: white;
        }
    }
}

/// Manages Slint UI rendering as an overlay on top of the OpenGL scene.
/// Uses software rendering to a pixel buffer, which is then uploaded to a GL texture.
pub struct EngineUiManager {
    window: Rc<MinimalSoftwareWindow>,
    _ui: MainWindow,
    pixel_buffer: Vec<PremultipliedRgbaColor>,
    width: u32,
    height: u32,
    last_cursor_pos: LogicalPosition,
    gl_texture: u32,
    needs_texture_resize: bool,
    overlay_vao: u32,
    overlay_vbo: u32,
}

impl EngineUiManager {
    /// Create a new EngineUiManager. Must be called BEFORE any other Slint components are created.
    pub fn new(width: u32, height: u32) -> Self {
        let window = init_slint_platform(width, height);

        let ui = MainWindow::new().unwrap();

        let pixel_count = (width * height) as usize;
        let pixel_buffer = vec![PremultipliedRgbaColor::default(); pixel_count];

        // Create GL texture with RGBA format
        let gl_texture = unsafe {
            let mut tex = 0u32;
            gl::GenTextures(1, &mut tex);
            gl::BindTexture(gl::TEXTURE_2D, tex);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as i32,
                width as i32,
                height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                std::ptr::null(),
            );
            gl::BindTexture(gl::TEXTURE_2D, 0);
            tex
        };

        // Create VAO/VBO for fullscreen quad overlay
        let (overlay_vao, overlay_vbo) = unsafe {
            let mut vao = 0u32;
            let mut vbo = 0u32;
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);

            // Fullscreen quad vertices: position (x,y) + texcoord (u,v)
            // note: Y is flipped for texture coordinates
            let quad_vertices: [f32; 24] = [
                // pos        // uv
                -1.0, 1.0, 0.0, 0.0, // top-left
                -1.0, -1.0, 0.0, 1.0, // bottom-left
                1.0, -1.0, 1.0, 1.0, // bottom-right
                -1.0, 1.0, 0.0, 0.0, // top-left
                1.0, -1.0, 1.0, 1.0, // bottom-right
                1.0, 1.0, 1.0, 0.0, // top-right
            ];

            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (quad_vertices.len() * std::mem::size_of::<f32>()) as isize,
                quad_vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            // position attribute (location 0)
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                4 * std::mem::size_of::<f32>() as i32,
                std::ptr::null(),
            );
            gl::EnableVertexAttribArray(0);

            // texcoord attribute (location 1)
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                4 * std::mem::size_of::<f32>() as i32,
                (2 * std::mem::size_of::<f32>()) as *const _,
            );
            gl::EnableVertexAttribArray(1);

            gl::BindVertexArray(0);
            (vao, vbo)
        };

        Self {
            window,
            _ui,
            pixel_buffer,
            width,
            height,
            last_cursor_pos: LogicalPosition::new(0.0, 0.0),
            gl_texture,
            needs_texture_resize: false,
            overlay_vao,
            overlay_vbo,
        }
    }

    /// Handle a winit window event. Returns true if Slint consumed the event.
    pub fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
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
                Some(match state {
                    winit::event::ElementState::Pressed => SlintWindowEvent::PointerPressed {
                        position: self.last_cursor_pos,
                        button: btn,
                    },
                    winit::event::ElementState::Released => SlintWindowEvent::PointerReleased {
                        position: self.last_cursor_pos,
                        button: btn,
                    },
                })
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => (*x * 20.0, *y * 20.0),
                    winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
                };
                Some(SlintWindowEvent::PointerScrolled {
                    position: self.last_cursor_pos,
                    delta_x: dx,
                    delta_y: dy,
                })
            }
            WindowEvent::Resized(size) => {
                self.resize(size.width, size.height);
                None // we handle resizing internally
            }
            // TODO: Add keyboard event handling
            _ => None,
        };

        if let Some(evt) = slint_event {
            self.window.dispatch_event(evt);
            // for now, we always assume Slint always processes events -
            // we can check if there's a focused element for more precise control
            true
        } else {
            false
        }
    }

    /// Resize the UI. Called automatically when WindowEvent::Resized is received.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.width = width;
        self.height = height;

        let pixel_count = (width * height) as usize;
        self.pixel_buffer
            .resize(pixel_count, PremultipliedRgbaColor::default());

        // notify Slint about the new size
        self.window.set_size(PhysicalSize::new(width, height));

        // mark texture for resize on next render
        self.needs_texture_resize = true;
    }

    /// Update Slint's internal timers and animations. Call this every frame.
    pub fn update(&mut self) {
        slint::platform::update_timers_and_animations();
    }

    /// Render the UI to the internal pixel buffer and upload to GL texture.
    /// Call this after update() but before drawing the overlay.
    pub fn render(&mut self) {
        // resize GL texture if needed
        if self.needs_texture_resize {
            unsafe {
                gl::BindTexture(gl::TEXTURE_2D, self.gl_texture);
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as i32,
                    self.width as i32,
                    self.height as i32,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    std::ptr::null(),
                );
                gl::BindTexture(gl::TEXTURE_2D, 0);
            }
            self.needs_texture_resize = false;
        }

        self.window.draw_if_needed(|renderer| {
            renderer.render(&mut self.pixel_buffer, self.width as usize);
        });

        // upload to GL texture
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.gl_texture);
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                self.width as i32,
                self.height as i32,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                self.pixel_buffer.as_ptr() as *const _,
            );
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }

    /// Draw the UI overlay on screen. Call this after render() and after all other rendering.
    /// This draws a fullscreen quad with the Slint UI texture blended on top.
    ///
    /// The shader should be the UiOverlay shader with `ui_texture` uniform set to texture unit 0.
    pub fn draw_overlay(&self, shader: &crate::shaders::Shader) {
        unsafe {
            // save current state
            let mut depth_test_enabled = 0i32;
            let mut blend_enabled = 0i32;
            gl::GetIntegerv(gl::DEPTH_TEST, &mut depth_test_enabled);
            gl::GetIntegerv(gl::BLEND, &mut blend_enabled);

            // set up for 2D overlay rendering
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::BLEND);
            // use premultiplied alpha blending since Slint uses PremultipliedRgbaColor
            gl::BlendFunc(gl::ONE, gl::ONE_MINUS_SRC_ALPHA);

            // activate shader and bind texture
            shader.activate();
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.gl_texture);

            // draw the fullscreen quad
            gl::BindVertexArray(self.overlay_vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
            gl::BindVertexArray(0);
            gl::BindTexture(gl::TEXTURE_2D, 0);

            // restore previous state
            if depth_test_enabled != 0 {
                gl::Enable(gl::DEPTH_TEST);
            }
            if blend_enabled == 0 {
                gl::Disable(gl::BLEND);
            }
        }
    }

    /// Get the GL texture ID for drawing as an overlay.
    pub fn texture(&self) -> u32 {
        self.gl_texture
    }

    /// Get the current UI size.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl Drop for EngineUiManager {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.gl_texture);
            gl::DeleteVertexArrays(1, &self.overlay_vao);
            gl::DeleteBuffers(1, &self.overlay_vbo);
        }
    }
}
