use std::collections::HashMap;

use image::GrayImage;
use rusttype::{point, Font, Scale};

use crate::{enums_types::ShaderType, gl_call, shaders::Shader};

use super::deprecated_game_ui::game_ui::Rect;

#[allow(dead_code)]
pub struct GlyphInfo {
    pub texture: u32,
    pub width: u32,
    pub height: u32,
    pub advance: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
}

pub struct FontManager {
    pub vao: u32,
    pub vbo: u32,
    pub glyphs: HashMap<char, GlyphInfo>,
    pub font_pixel_size: f32,
}

impl FontManager {
    pub fn new() -> Self {
        Self {
            vao: 0,
            vbo: 0,
            glyphs: HashMap::new(),
            font_pixel_size: 96.0,
        }
    }

    pub fn load_chars(&mut self, phrase: &str) {
        // let font_data = include_bytes!("../../resources/fonts/JetBrainsMonoNL-Regular.ttf");
        // let font_data = include_bytes!("../../resources/fonts/VintageCorner-Regular.otf");
        // let font_data = include_bytes!("../../resources/fonts/Moodnight.otf");
        let font_data = include_bytes!("../../resources/fonts/BebasNeue-Regular.ttf");

        let font = Font::try_from_bytes(font_data).unwrap();
        let scale = Scale::uniform(self.font_pixel_size);
        let v_metrics = font.v_metrics(scale);


        for c in phrase.chars() {
            if self.glyphs.contains_key(&c) {
                continue;
            }

            let glyph = font.glyph(c).scaled(scale).positioned(point(0.0, v_metrics.ascent));
            let h_metrics = glyph.unpositioned().h_metrics();
            let advance = h_metrics.advance_width;

            if let Some(bb) = glyph.pixel_bounding_box() {
                let bearing_x = bb.min.x as f32;
                let bearing_y = bb.min.y as f32;

                let width = bb.width() as usize;
                let height = bb.height() as usize;

                let mut pixel_data = vec![0u8; width * height];

                glyph.draw(|x, y, v| {
                    let index = (y as usize * width) + x as usize;
                    pixel_data[index] = (v * 255.0) as u8;
                });

                let mut tex: u32 = 0;
                unsafe {
                    gl::GenTextures(1, &mut tex);
                    gl::BindTexture(gl::TEXTURE_2D, tex);
                    gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
                    gl::TexImage2D(
                        gl::TEXTURE_2D,
                        0,
                        gl::RED as i32,
                        width as i32,
                        height as i32,
                        0,
                        gl::RED,
                        gl::UNSIGNED_BYTE,
                        pixel_data.as_ptr() as *const _,
                    );
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
                }

                self.glyphs.insert(c, GlyphInfo {
                    texture: tex,
                    width: width as u32,
                    height: height as u32,
                    advance,
                    bearing_x,
                    bearing_y: -bearing_y,
                });
            } else {
                // Handle space or invisible glyphs
                self.glyphs.insert(c, GlyphInfo {
                    texture: 0, // No texture needed
                    width: 0,
                    height: 0,
                    advance,
                    bearing_x: 0.0,
                    bearing_y: 0.0,
                });
            }
        }
    }

    pub fn setup_buffers(&mut self) {
        unsafe {
            gl::GenVertexArrays(1, &mut self.vao);
            gl::GenBuffers(1, &mut self.vbo);

            gl::BindVertexArray(self.vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (5 * 6 * std::mem::size_of::<f32>()) as isize, // 6 vertices, 5 floats each
                std::ptr::null(),
                gl::DYNAMIC_DRAW,
            );

            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 5 * std::mem::size_of::<f32>() as i32, std::ptr::null());
            gl::EnableVertexAttribArray(0);

            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, 5 * std::mem::size_of::<f32>() as i32, (3 * std::mem::size_of::<f32>()) as *const _);
            gl::EnableVertexAttribArray(1);

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }
    }

    #[allow(dead_code)]
    pub fn render_phrase(&self, phrase: &str, x: f32, y: f32, fb_width: f32, fb_height: f32, shader: &Shader, scale: f32) {
        shader.activate();

        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::BindVertexArray(self.vao);
        }

        let mut cursor_x = x;

        for c in phrase.chars() {
            if let Some(glyph) = self.glyphs.get(&c) {
                let sx = 2.0 / fb_width;
                let sy = 2.0 / fb_height;

                let w = glyph.width as f32 * scale;
                let h = glyph.height as f32 * scale;

                let xpos = cursor_x + glyph.bearing_x * scale;
                let ypos = y - glyph.bearing_y * scale;

                let x0 = xpos * sx - 1.0;
                let x1 = (xpos + w) * sx - 1.0;
                let y0 = 1.0 - ypos * sy;
                let y1 = 1.0 - (ypos + h) * sy;

                let vertices: [f32; 30] = [
                    x0, y0, 0.0, 0.0, 0.0,
                    x0, y1, 0.0, 0.0, 1.0,
                    x1, y1, 0.0, 1.0, 1.0,
                    x0, y0, 0.0, 0.0, 0.0,
                    x1, y1, 0.0, 1.0, 1.0,
                    x1, y0, 0.0, 1.0, 0.0,
                ];

                unsafe {
                    gl::ActiveTexture(gl::TEXTURE1);
                    gl::BindTexture(gl::TEXTURE_2D, glyph.texture);
                    gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
                    gl::BufferSubData(
                        gl::ARRAY_BUFFER,
                        0,
                        (vertices.len() * std::mem::size_of::<f32>()) as isize,
                        vertices.as_ptr() as *const _,
                    );
                    gl::DrawArrays(gl::TRIANGLES, 0, 6);
                }

                cursor_x += (glyph.advance) * scale;
                // cursor_x += ( as f32) * scale; // Hard coded letter spacing of 2.0
            }
        }

        unsafe {
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::BindVertexArray(0);
            gl::Disable(gl::BLEND);
        }
    }

    #[allow(dead_code)]
    pub fn render_phrase_centered(
        &self,
        phrase: &str,
        rect: &Rect,
        fb_width: f32,
        fb_height: f32,
        shader: &Shader,
        scale: f32,
    ) {
        // Compute total width
        let mut text_width = 0.0;
        for c in phrase.chars() {
            if let Some(glyph) = self.glyphs.get(&c) {
                text_width += glyph.advance * scale;
            }
        }

        let text_x = rect.x + (rect.w - text_width) / 2.0;

        let first_glyph = phrase
            .chars()
            .filter_map(|c| self.glyphs.get(&c))
            .next();

        let text_y = if let Some(g) = first_glyph {
            rect.y + (rect.h / 2.0) + ((g.bearing_y - g.height as f32 / 2.0) * scale)
        } else {
            rect.y + rect.h / 2.0
        };

        self.render_phrase(phrase, text_x, text_y, fb_width, fb_height, shader, scale);
    }

}

