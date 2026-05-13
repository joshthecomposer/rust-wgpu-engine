use glam::Vec3;

use crate::util::constants::MAX_BONE_INFLUENCE;

#[repr(C)]
#[derive(Debug, Clone, bytemuck::Pod, bytemuck::Zeroable, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],

    pub bone_ids: [i32; MAX_BONE_INFLUENCE],
    pub bone_weights: [f32; MAX_BONE_INFLUENCE],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Sint32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }

    pub fn boneless(position: Vec3, normal: Vec3) -> Self {
        Self {
            position: position.into(),
            normal: normal.into(),
            uv: [0.0, 0.0],

            bone_ids: [-1; MAX_BONE_INFLUENCE],
            bone_weights: [0.0; MAX_BONE_INFLUENCE],
        }
    }
}
