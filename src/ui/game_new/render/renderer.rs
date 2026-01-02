use std::mem;
use std::ptr;

use super::batch::RenderBatch;
use super::vertex::UiVertex;
use crate::gl_call;
use crate::ui::game_new::font_system::FontSystem;

use crate::shaders::Shader;
use crate::ui::game_new::styles::Rect;
use glyph_brush::{ab_glyph::PxScale, BrushAction, BrushError, Section, Text};

#[derive(Clone, Copy, Debug)]
pub struct UiGlyph {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
    color: [f32; 4],
}

pub struct UiRenderer {
    shader: Shader,
    vao: u32,
    vbo: u32,
    ebo: u32,
    batch: RenderBatch,
    screen_width: f32,
    screen_height: f32,

    // Text Rendering
    white_texture: u32,
    font_texture: u32,
    queued_text: Vec<(String, f32, f32, f32, [f32; 4], Option<String>)>, // (text, x, y, font_size, color, font_family)
    cached_glyphs: Vec<UiGlyph>, // cache last successfully rendered glyphs
}

impl UiRenderer {
    pub fn new() -> Self {
        let shader = Shader::new("resources/shaders/custom_ui.glsl");

        let mut vao = 0;
        let mut vbo = 0;
        let mut ebo = 0;

        unsafe {
            gl_call!(gl::GenVertexArrays(1, &mut vao));
            gl_call!(gl::GenBuffers(1, &mut vbo));
            gl_call!(gl::GenBuffers(1, &mut ebo));

            gl_call!(gl::BindVertexArray(vao));

            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, vbo));
            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                (16384 * mem::size_of::<UiVertex>()) as isize, // Increased buffer size
                ptr::null(),
                gl::DYNAMIC_DRAW
            ));

            gl_call!(gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo));
            gl_call!(gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (32768 * mem::size_of::<u32>()) as isize, // Increased buffer size
                ptr::null(),
                gl::DYNAMIC_DRAW
            ));

            let stride = mem::size_of::<UiVertex>() as i32;

            // position attribute (location = 0)
            gl_call!(gl::EnableVertexAttribArray(0));
            gl_call!(gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                ptr::null()
            ));

            // color attribute (location = 1)
            gl_call!(gl::EnableVertexAttribArray(1));
            gl_call!(gl::VertexAttribPointer(
                1,
                4,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (2 * mem::size_of::<f32>()) as *const _
            ));

            // uv attribute (location = 2)
            gl_call!(gl::EnableVertexAttribArray(2));
            gl_call!(gl::VertexAttribPointer(
                2,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (6 * mem::size_of::<f32>()) as *const _
            ));

            gl_call!(gl::BindVertexArray(0));
        }

        // initialize font texture
        let mut white_texture = 0;
        unsafe {
            gl_call!(gl::GenTextures(1, &mut white_texture));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, white_texture));
            let white_pixel: [u8; 4] = [255, 255, 255, 255];
            gl_call!(gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as i32,
                1,
                1,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                white_pixel.as_ptr() as *const _
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                gl::NEAREST as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAG_FILTER,
                gl::NEAREST as i32
            ));
        }

        Self {
            shader,
            vao,
            vbo,
            ebo,
            batch: RenderBatch::new(),
            screen_width: 1920.0,
            screen_height: 1080.0,
            white_texture,
            font_texture: 0,
            queued_text: Vec::new(),
            cached_glyphs: Vec::new(),
        }
    }

    /// sets the screen dimensions for the renderer
    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
    }

    /// prepares the renderer for a new frame by clearing the current batch and queued text
    pub fn begin(&mut self) {
        self.batch.clear();
        self.queued_text.clear();
    }

    pub fn draw_rect(&mut self, rect: Rect, color: [f32; 4]) {
        self.batch.push_rect(rect, color);
    }

    /// queues text to be rendered at the specified position with the given size and color
    pub fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: [f32; 4],
        font_family: Option<String>,
    ) {
        self.queued_text
            .push((text.to_string(), x, y, font_size, color, font_family));
    }

    /// Flushes the current render batch to the GPU using the specified texture.
    ///
    /// This method uploads vertex and index data, configures the shader, sets up
    /// the necessary OpenGL state (blending, depth, viewport), and executes the draw call.
    fn flush(&mut self, texture_id: u32) {
        if self.batch.is_empty() {
            return;
        }

        unsafe {
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo));
            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                (self.batch.vertices.len() * mem::size_of::<UiVertex>()) as isize,
                self.batch.vertices.as_ptr() as *const _,
                gl::DYNAMIC_DRAW
            ));

            gl_call!(gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebo));
            gl_call!(gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (self.batch.indices.len() * mem::size_of::<u32>()) as isize,
                self.batch.indices.as_ptr() as *const _,
                gl::DYNAMIC_DRAW
            ));
        }

        self.shader.activate();
        self.shader
            .set_vec2("u_screen_size", self.screen_width, self.screen_height);

        unsafe {
            gl_call!(gl::ActiveTexture(gl::TEXTURE0));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, texture_id));
            self.shader.set_int("u_texture", 0);
            self.shader
                .set_bool("u_is_alpha_mask", texture_id == self.font_texture);

            gl_call!(gl::Viewport(
                0,
                0,
                self.screen_width as i32,
                self.screen_height as i32
            ));
            gl_call!(gl::Disable(gl::DEPTH_TEST));
            gl_call!(gl::DepthMask(gl::FALSE));
            gl_call!(gl::Disable(gl::CULL_FACE));
            gl_call!(gl::Disable(gl::SCISSOR_TEST));

            gl_call!(gl::Enable(gl::BLEND));
            gl_call!(gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA));

            gl_call!(gl::BindVertexArray(self.vao));
            gl_call!(gl::DrawElements(
                gl::TRIANGLES,
                self.batch.indices.len() as i32,
                gl::UNSIGNED_INT,
                ptr::null()
            ));
            gl_call!(gl::BindVertexArray(0));

            gl_call!(gl::DepthMask(gl::TRUE));
            gl_call!(gl::Enable(gl::DEPTH_TEST));
        }

        self.batch.clear();
    }

    /// Ends the current frame by flushing the rectangle batch and processing/flushing queued text.
    /// Ends the current frame by flushing the rectangle batch and processing/flushing queued text.
    pub fn end(&mut self, font_system: &mut FontSystem) {
        self.flush(self.white_texture);

        // queue all text
        for (text, x, y, font_size, color, font_family) in self.queued_text.drain(..) {
            let font_id = font_system.get_font_id(font_family.as_deref());
            font_system.glyph_brush.queue(Section {
                screen_position: (x, y),
                bounds: (self.screen_width, self.screen_height),
                text: vec![Text::new(&text)
                    .with_scale(PxScale::from(font_size))
                    .with_color(color)
                    .with_font_id(font_id)],
                ..Section::default()
            });
        }

        let mut font_texture = self.font_texture;
        let (cache_width, cache_height) = font_system.glyph_brush.texture_dimensions();
        let mut loop_iteration = 0;
        const MAX_ITERATIONS: usize = 10;

        loop {
            loop_iteration += 1;
            if loop_iteration > MAX_ITERATIONS {
                eprintln!("[UiRenderer] Warning: process_queued loop reached max iterations ({}), breaking to prevent infinite loop", MAX_ITERATIONS);
                break;
            }

            let result = font_system.glyph_brush.process_queued(
                |rect, tex_data| unsafe {
                    if font_texture == 0 {
                        gl_call!(gl::GenTextures(1, &mut font_texture));
                        gl_call!(gl::BindTexture(gl::TEXTURE_2D, font_texture));
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

                        gl_call!(gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1));
                        gl_call!(gl::TexImage2D(
                            gl::TEXTURE_2D,
                            0,
                            gl::R8 as i32,
                            cache_width as i32,
                            cache_height as i32,
                            0,
                            gl::RED,
                            gl::UNSIGNED_BYTE,
                            ptr::null()
                        ));
                        gl_call!(gl::PixelStorei(gl::UNPACK_ALIGNMENT, 4));
                    }
                    gl_call!(gl::BindTexture(gl::TEXTURE_2D, font_texture));
                    gl_call!(gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1));
                    gl_call!(gl::TexSubImage2D(
                        gl::TEXTURE_2D,
                        0,
                        rect.min[0] as i32,
                        rect.min[1] as i32,
                        rect.width() as i32,
                        rect.height() as i32,
                        gl::RED,
                        gl::UNSIGNED_BYTE,
                        tex_data.as_ptr() as *const _
                    ));
                    gl_call!(gl::PixelStorei(gl::UNPACK_ALIGNMENT, 4));
                },
                |vertex| {
                    let uv = vertex.tex_coords;
                    let color = vertex.extra.color;
                    let rect = vertex.pixel_coords;

                    UiGlyph {
                        x0: rect.min.x,
                        y0: rect.min.y,
                        x1: rect.max.x,
                        y1: rect.max.y,
                        u0: uv.min.x,
                        v0: uv.min.y,
                        u1: uv.max.x,
                        v1: uv.max.y,
                        color,
                    }
                },
            );

            match result {
                Ok(BrushAction::Draw(glyphs)) => {
                    self.cached_glyphs = glyphs.iter().copied().collect();

                    for glyph in glyphs {
                        let idx = self.batch.vertices.len() as u32;
                        self.batch.vertices.push(UiVertex::new(
                            glyph.x0,
                            glyph.y0,
                            glyph.color,
                            [glyph.u0, glyph.v0],
                        ));
                        self.batch.vertices.push(UiVertex::new(
                            glyph.x1,
                            glyph.y0,
                            glyph.color,
                            [glyph.u1, glyph.v0],
                        ));
                        self.batch.vertices.push(UiVertex::new(
                            glyph.x1,
                            glyph.y1,
                            glyph.color,
                            [glyph.u1, glyph.v1],
                        ));
                        self.batch.vertices.push(UiVertex::new(
                            glyph.x0,
                            glyph.y1,
                            glyph.color,
                            [glyph.u0, glyph.v1],
                        ));
                        self.batch.indices.extend_from_slice(&[
                            idx,
                            idx + 1,
                            idx + 2,
                            idx,
                            idx + 2,
                            idx + 3,
                        ]);
                    }
                    break;
                }
                Ok(BrushAction::ReDraw) => {
                    if !self.cached_glyphs.is_empty() {
                        for glyph in &self.cached_glyphs {
                            let idx = self.batch.vertices.len() as u32;
                            self.batch.vertices.push(UiVertex::new(
                                glyph.x0,
                                glyph.y0,
                                glyph.color,
                                [glyph.u0, glyph.v0],
                            ));
                            self.batch.vertices.push(UiVertex::new(
                                glyph.x1,
                                glyph.y0,
                                glyph.color,
                                [glyph.u1, glyph.v0],
                            ));
                            self.batch.vertices.push(UiVertex::new(
                                glyph.x1,
                                glyph.y1,
                                glyph.color,
                                [glyph.u1, glyph.v1],
                            ));
                            self.batch.vertices.push(UiVertex::new(
                                glyph.x0,
                                glyph.y1,
                                glyph.color,
                                [glyph.u0, glyph.v1],
                            ));
                            self.batch.indices.extend_from_slice(&[
                                idx,
                                idx + 1,
                                idx + 2,
                                idx,
                                idx + 2,
                                idx + 3,
                            ]);
                        }
                    }
                    break;
                }
                Err(BrushError::TextureTooSmall { suggested }) => {
                    let (new_width, new_height) = suggested;
                    unsafe {
                        if font_texture != 0 {
                            gl_call!(gl::DeleteTextures(1, &font_texture));
                        }
                        gl_call!(gl::GenTextures(1, &mut font_texture));
                        gl_call!(gl::BindTexture(gl::TEXTURE_2D, font_texture));
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

                        gl_call!(gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1));
                        gl_call!(gl::TexImage2D(
                            gl::TEXTURE_2D,
                            0,
                            gl::R8 as i32,
                            new_width as i32,
                            new_height as i32,
                            0,
                            gl::RED,
                            gl::UNSIGNED_BYTE,
                            ptr::null()
                        ));
                        self.font_texture = font_texture;
                        gl_call!(gl::PixelStorei(gl::UNPACK_ALIGNMENT, 4));
                    }
                }
            }
        }

        self.font_texture = font_texture;
        self.flush(self.font_texture);
    }
}

impl Drop for UiRenderer {
    fn drop(&mut self) {
        unsafe {
            gl_call!(gl::DeleteVertexArrays(1, &self.vao));
            gl_call!(gl::DeleteBuffers(1, &self.vbo));
            gl_call!(gl::DeleteBuffers(1, &self.ebo));
            if self.white_texture != 0 {
                gl_call!(gl::DeleteTextures(1, &self.white_texture));
            }
            if self.font_texture != 0 {
                gl_call!(gl::DeleteTextures(1, &self.font_texture));
            }
        }
    }
}

impl Default for UiRenderer {
    fn default() -> Self {
        Self::new()
    }
}
