use glam::{Vec2, Vec3, Vec4};

use crate::util::constants::MAX_BONE_INFLUENCE;

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
    pub texture: Option<Texture>,

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
            texture: None,

            directory: String::new(),
            full_path: String::new(),

            color_for_texture: false,
        }
    }
}
