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
    queued_text: Vec<(
        String,
        f32,
        f32,
        f32,
        [f32; 4],
        Option<String>,
        Option<[i32; 4]>,
    )>, // (text, x, y, font_size, color, font_family, scissor)
    cached_glyphs: Vec<UiGlyph>, // cache last successfully rendered glyphs
    active_texture: u32,

    // scissor clipping (for ScrollView)
    scissor_stack: Vec<[i32; 4]>, // [x, y, width, height] in GL coordinates
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
            active_texture: white_texture,
            scissor_stack: Vec::new(),
        }
    }

    /// sets the screen dimensions for the renderer
    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
    }

    /// Prepares the renderer for a new frame by clearing the current batch and queued text
    pub fn begin(&mut self) {
        self.batch.clear();
        self.queued_text.clear();
        self.active_texture = self.white_texture;
        self.scissor_stack.clear();
    }

    /// Pushes a scissor rect onto the stack for clipping.
    /// UI coordinates (top-left origin) are converted to GL coordinates (bottom-left origin).
    /// If there's already a scissor rect, the new rect is intersected with it.
    pub fn push_scissor(&mut self, rect: Rect) {
        // Flush pending draws before changing scissor state
        self.flush(self.active_texture);

        // Convert UI coordinates (top-left origin) to GL coordinates (bottom-left origin)
        let gl_x = rect.x as i32;
        let gl_y = (self.screen_height - rect.y - rect.height) as i32;
        let gl_w = rect.width as i32;
        let gl_h = rect.height as i32;

        let new_scissor = if let Some(current) = self.scissor_stack.last() {
            // Intersect with current scissor rect for nested clipping
            Self::intersect_rects(*current, [gl_x, gl_y, gl_w, gl_h])
        } else {
            [gl_x, gl_y, gl_w, gl_h]
        };

        self.scissor_stack.push(new_scissor);
    }

    /// Pops the current scissor rect from the stack, restoring the previous state.
    pub fn pop_scissor(&mut self) {
        // Flush pending draws before changing scissor state
        self.flush(self.active_texture);
        self.scissor_stack.pop();
    }

    /// Intersects two rects in GL coordinates. Returns a rect that is the intersection.
    fn intersect_rects(a: [i32; 4], b: [i32; 4]) -> [i32; 4] {
        let x = a[0].max(b[0]);
        let y = a[1].max(b[1]);
        let right = (a[0] + a[2]).min(b[0] + b[2]);
        let top = (a[1] + a[3]).min(b[1] + b[3]);
        [x, y, (right - x).max(0), (top - y).max(0)]
    }

    pub fn draw_rect(&mut self, rect: Rect, color: [f32; 4]) {
        if self.active_texture != self.white_texture {
            self.flush(self.active_texture);
            self.active_texture = self.white_texture;
        }
        self.batch.push_rect(rect, color);
    }

    pub fn draw_textured_rect(&mut self, rect: Rect, texture_id: u32, color: Option<[f32; 4]>) {
        if self.active_texture != texture_id {
            self.flush(self.active_texture);
            self.active_texture = texture_id;
        }
        let color = color.unwrap_or([1.0, 1.0, 1.0, 1.0]);
        // Default standard UVs for a full texture
        let uv = [0.0, 0.0, 1.0, 1.0];
        self.batch.push_textured_rect(rect, uv, color);
    }

    /// queues text to be rendered at the specified position with the given size and color
    pub fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: [f32; 4],
        font_family: Option<&str>,
    ) {
        let scissor = self.scissor_stack.last().copied();
        self.queued_text.push((
            text.to_string(),
            x,
            y,
            font_size,
            color,
            font_family.map(|s| s.to_string()),
            scissor,
        ));
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

            // Respect current scissor state (for ScrollView clipping)
            if let Some(scissor) = self.scissor_stack.last() {
                gl_call!(gl::Enable(gl::SCISSOR_TEST));
                gl_call!(gl::Scissor(scissor[0], scissor[1], scissor[2], scissor[3]));
            } else {
                gl_call!(gl::Disable(gl::SCISSOR_TEST));
            }

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
        self.flush(self.active_texture);

        // Collect all text to avoid borrowing self.queued_text
        let all_text: Vec<_> = self.queued_text.drain(..).collect();

        if all_text.is_empty() {
            return;
        }

        // Sort text by scissor state to minimize state changes
        // This assumes Option<[i32; 4]> can be compared, which it can.
        let mut sorted_text = all_text;
        sorted_text.sort_by_key(|(_, _, _, _, _, _, scissor)| *scissor);

        let mut current_scissor = sorted_text[0].6;
        let mut batch_started = false;

        for (text, x, y, font_size, color, font_family, scissor) in sorted_text {
            // If scissor state changes, flush the previous batch
            if scissor != current_scissor {
                if batch_started {
                    self.render_text_batch(font_system, current_scissor);
                }
                current_scissor = scissor;
                batch_started = false;
            }

            // Queue text to glyph brush
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
            batch_started = true;
        }

        // Flush final batch
        if batch_started {
            self.render_text_batch(font_system, current_scissor);
        }

        // Ensure scissor test is disabled and stack is cleared after UI rendering
        self.scissor_stack.clear();
        unsafe {
            gl_call!(gl::Disable(gl::SCISSOR_TEST));
        }
    }

    /// Helper to process currently queued text in glyph_brush and render it with a specific scissor
    fn render_text_batch(&mut self, font_system: &mut FontSystem, scissor: Option<[i32; 4]>) {
        if let Some(s) = scissor {
            self.scissor_stack.push(s);
        }

        let mut font_texture = self.font_texture;
        let mut loop_iteration = 0;
        const MAX_ITERATIONS: usize = 100;

        loop {
            loop_iteration += 1;
            if loop_iteration > MAX_ITERATIONS {
                eprintln!("[UiRenderer] Warning: process_queued loop reached max iterations ({}), breaking", MAX_ITERATIONS);
                break;
            }

            let (cache_width, cache_height) = font_system.glyph_brush.texture_dimensions();

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
                Ok(BrushAction::Draw(vertices)) => {
                    self.cached_glyphs = vertices.iter().copied().collect();

                    for glyph in vertices {
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

                    // CRITICAL FIX: Tell glyph_brush that the texture is being resized!
                    font_system
                        .glyph_brush
                        .resize_texture(new_width, new_height);

                    unsafe {
                        if font_texture != 0 {
                            gl_call!(gl::DeleteTextures(1, &font_texture));
                        }
                        font_texture = 0;
                    }
                }
            }
        }

        self.font_texture = font_texture;
        self.flush(self.font_texture);

        if scissor.is_some() {
            self.scissor_stack.pop();
        }
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
