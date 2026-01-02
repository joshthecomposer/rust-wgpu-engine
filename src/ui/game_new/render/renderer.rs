use std::mem;
use std::ptr;

use crate::gl_call;
use crate::shaders::Shader;
use crate::ui::game_new::styles::Rect;

use super::batch::RenderBatch;
use super::vertex::UiVertex;

pub struct UiRenderer {
    shader: Shader,
    vao: u32,
    vbo: u32,
    ebo: u32,
    batch: RenderBatch,
    screen_width: f32,
    screen_height: f32,
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
                (4096 * mem::size_of::<UiVertex>()) as isize,
                ptr::null(),
                gl::DYNAMIC_DRAW
            ));

            gl_call!(gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo));
            gl_call!(gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (8192 * mem::size_of::<u32>()) as isize,
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

            gl_call!(gl::BindVertexArray(0));
        }

        Self {
            shader,
            vao,
            vbo,
            ebo,
            batch: RenderBatch::new(),
            screen_width: 1920.0,
            screen_height: 1080.0,
        }
    }

    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
    }

    pub fn begin(&mut self) {
        self.batch.clear();
    }

    pub fn draw_rect(&mut self, rect: Rect, color: [f32; 4]) {
        self.batch.push_rect(rect, color);
    }

    pub fn end(&mut self) {
        if self.batch.is_empty() {
            return;
        }

        unsafe {
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo));
            gl_call!(gl::BufferSubData(
                gl::ARRAY_BUFFER,
                0,
                (self.batch.vertices.len() * mem::size_of::<UiVertex>()) as isize,
                self.batch.vertices.as_ptr() as *const _
            ));

            gl_call!(gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebo));
            gl_call!(gl::BufferSubData(
                gl::ELEMENT_ARRAY_BUFFER,
                0,
                (self.batch.indices.len() * mem::size_of::<u32>()) as isize,
                self.batch.indices.as_ptr() as *const _
            ));
        }

        self.shader.activate();
        self.shader
            .set_vec2("u_screen_size", self.screen_width, self.screen_height);

        unsafe {
            gl_call!(gl::Viewport(
                0,
                0,
                self.screen_width as i32,
                self.screen_height as i32
            ));
            gl_call!(gl::Disable(gl::DEPTH_TEST));
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

            gl_call!(gl::Enable(gl::DEPTH_TEST));
        }
    }
}

impl Drop for UiRenderer {
    fn drop(&mut self) {
        unsafe {
            gl_call!(gl::DeleteVertexArrays(1, &self.vao));
            gl_call!(gl::DeleteBuffers(1, &self.vbo));
            gl_call!(gl::DeleteBuffers(1, &self.ebo));
        }
    }
}

impl Default for UiRenderer {
    fn default() -> Self {
        Self::new()
    }
}
