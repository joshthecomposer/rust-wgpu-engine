use std::mem::size_of;

use crate::{
    enums_types::InstanceUniform,
    wgpu_backend::{
        pipelines::{create_render_pipeline, shared::SharedLayouts},
        vertex::Vertex,
    },
};

const MAX_STATIC_INSTANCES: u64 = 10_000;

pub struct StaticModelResources {
    pub pipeline: wgpu::RenderPipeline,
    pub instance_buffer: wgpu::Buffer,
}

pub fn build(
    device: &wgpu::Device,
    shared: &SharedLayouts,
    color_format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
) -> StaticModelResources {
    let shader = wgpu::ShaderModuleDescriptor {
        label: Some("Static Model Shader"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("../../../resources/shaders/model/static_model.wgsl").into(),
        ),
    };

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Static Model Pipeline Layout"),
        bind_group_layouts: &[Some(&shared.texture), Some(&shared.camera)],
        immediate_size: 0,
    });

    let pipeline = create_render_pipeline(
        device,
        &pipeline_layout,
        color_format,
        Some(depth_format),
        &[Vertex::desc(), InstanceUniform::desc()],
        shader,
        Some("Static Model Pipeline"),
        Some(wgpu::CompareFunction::Less),
        Some(wgpu::Face::Back),
    );

    let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("static_instance_buffer"),
        size: MAX_STATIC_INSTANCES * size_of::<InstanceUniform>() as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
        mapped_at_creation: false,
    });

    StaticModelResources {
        pipeline,
        instance_buffer,
    }
}
