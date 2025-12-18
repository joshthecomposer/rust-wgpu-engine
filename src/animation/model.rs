use std::{
    mem::{self, offset_of},
    ptr,
};

use glam::{Vec2, Vec3, Vec4};

use crate::{gl_call, shaders::Shader, util::constants::MAX_BONE_INFLUENCE};

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub base_color: Vec4,

    pub bone_ids: [i32; MAX_BONE_INFLUENCE],
    pub bone_weights: [f32; MAX_BONE_INFLUENCE],
}

impl Vertex {
    pub fn new(position: Vec3, normal: Vec3) -> Self {
        Self {
            position,
            normal,
            uv: Vec2::new(0.0, 0.0),
            base_color: Vec4::new(1.0, 0.0, 0.0, 1.0),

            bone_ids: [-1; MAX_BONE_INFLUENCE],
            bone_weights: [0.0; MAX_BONE_INFLUENCE],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Texture {
    pub id: u32,
    pub _type: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct Model {
    pub vao: u32,
    pub vbo: u32,
    pub ebo: u32,

    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub textures: [Option<Texture>; 9],

    pub directory: String,
    pub full_path: String,

    pub color_for_texture: bool,
}

impl Model {
    pub fn new() -> Self {
        Self {
            vao: 0,
            vbo: 0,
            ebo: 0,

            vertices: vec![],
            indices: vec![],
            textures: [None, None, None, None, None, None, None, None, None],

            directory: String::new(),
            full_path: String::new(),

            color_for_texture: false,
        }
    }

    /// Get a texture by index (0 = Diffuse, 1 = Specular, etc.)
    pub fn get_tex(&self, index: usize) -> Option<&Texture> {
        if index < self.textures.len() {
            self.textures[index].as_ref()
        } else {
            None
        }
    }

    /// Convenience: get by "type" using a fixed mapping
    pub fn get_tex_by_type(&self, tex_type: &str) -> Option<&Texture> {
        match tex_type {
            "Diffuse" => self.textures[0].as_ref(),
            "Specular" => self.textures[1].as_ref(),
            "Emissive" => self.textures[2].as_ref(),
            "Opacity" => self.textures[3].as_ref(),
            _ => None,
        }
    }

    pub fn setup_opengl(&mut self) {
        unsafe {
            gl_call!(gl::GenVertexArrays(1, &mut self.vao));
            gl_call!(gl::GenBuffers(1, &mut self.vbo));
            gl_call!(gl::GenBuffers(1, &mut self.ebo));

            gl_call!(gl::BindVertexArray(self.vao));
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo));

            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                (mem::size_of::<Vertex>() * self.vertices.len()) as isize,
                self.vertices.as_ptr().cast(),
                gl::STATIC_DRAW,
            ));

            gl_call!(gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebo));
            gl_call!(gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (mem::size_of::<u32>() * self.indices.len()) as isize,
                self.indices.as_ptr().cast(),
                gl::STATIC_DRAW
            ));

            gl_call!(gl::EnableVertexAttribArray(0));
            gl_call!(gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                ptr::null(),
            ));

            gl_call!(gl::EnableVertexAttribArray(1));
            gl_call!(gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, normal) as *const _
            ));

            gl_call!(gl::EnableVertexAttribArray(2));
            gl_call!(gl::VertexAttribPointer(
                2,
                2,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, uv) as *const _
            ));

            gl_call!(gl::EnableVertexAttribArray(3));
            gl_call!(gl::VertexAttribPointer(
                3,
                4,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, base_color) as *const _
            ));

            gl_call!(gl::EnableVertexAttribArray(4));
            gl_call!(gl::VertexAttribIPointer(
                4,
                4,
                gl::INT,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, bone_ids) as *const _
            ));

            gl_call!(gl::EnableVertexAttribArray(5));
            gl_call!(gl::VertexAttribPointer(
                5,
                4,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, bone_weights) as *const _
            ));

            gl::BindVertexArray(0);
        }
    }

    pub fn draw(&self, shader: &mut Shader) {
        if self.color_for_texture {
            shader.set_bool("use_base_color", true);
            shader.set_bool("has_opacity_texture", false);
        } else {
            shader.set_bool("use_base_color", false);
            if let Some(diff) = self.get_tex(1) {
                // Diffuse
                unsafe {
                    gl::ActiveTexture(gl::TEXTURE1);
                    gl::BindTexture(gl::TEXTURE_2D, diff.id);
                }
            }
            if let Some(spec) = self.get_tex(2) {
                // Specular
                unsafe {
                    gl::ActiveTexture(gl::TEXTURE2);
                    gl::BindTexture(gl::TEXTURE_2D, spec.id);
                }
            }
            if let Some(emis) = self.get_tex(3) {
                // Emissive
                unsafe {
                    gl::ActiveTexture(gl::TEXTURE3);
                    gl::BindTexture(gl::TEXTURE_2D, emis.id);
                }
            }
            if let Some(opac) = self.get_tex(8) {
                shader.set_bool("has_opacity_texture", true);
                unsafe {
                    gl::ActiveTexture(gl::TEXTURE4);
                    gl::BindTexture(gl::TEXTURE_2D, opac.id);
                }
            } else {
                shader.set_bool("has_opacity_texture", false);
            }
        }

        unsafe {
            gl_call!(gl::BindVertexArray(self.vao));
            gl_call!(gl::DrawElements(
                gl::TRIANGLES,
                self.indices.len() as i32,
                gl::UNSIGNED_INT,
                ptr::null(),
            ));

            shader.set_bool("has_opacity_texture", false);
            shader.set_bool("use_base_color", false);
            gl_call!(gl::BindVertexArray(0));
        }
    }
}
