use crate::wgpu_backend::{material::Material, vertex::Vertex};

#[derive(Clone)]
pub struct Model {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,

    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,

    pub num_elements: u32,

    pub material: Material,

    pub directory: String,
    pub full_path: String,
}
