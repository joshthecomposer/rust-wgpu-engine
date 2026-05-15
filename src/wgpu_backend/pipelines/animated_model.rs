use std::{mem::size_of, num::NonZeroU64};

use crate::{
    enums_types::InstanceUniform,
    wgpu_backend::{
        bone_uniforms::BoneUniforms,
        pipelines::{create_render_pipeline, shared::SharedLayouts},
        vertex::Vertex,
    },
};

const MAX_ANIMATED_DRAWS: u64 = 256;

pub struct AnimatedModelResources {
    pub pipeline: wgpu::RenderPipeline,
    pub instance_buffer: wgpu::Buffer,
    pub bones_layout: wgpu::BindGroupLayout,
    pub bones_buffer: wgpu::Buffer,
    pub bones_bind_group: wgpu::BindGroup,
}

pub fn build(
    device: &wgpu::Device,
    shared: &SharedLayouts,
    color_format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
) -> AnimatedModelResources {
    let bone_uniform_size =
        NonZeroU64::new(size_of::<BoneUniforms>() as u64).expect("BoneUniforms must be non-empty");
    let bone_buffer_size = bone_uniform_size.get() * MAX_ANIMATED_DRAWS;

    let bones_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bone_uniform_buffer"),
        size: bone_buffer_size,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bones_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bones_bind_group_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: Some(bone_uniform_size),
            },
            count: None,
        }],
    });

    let bones_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bones_bind_group"),
        layout: &bones_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &bones_buffer,
                offset: 0,
                size: Some(bone_uniform_size),
            }),
        }],
    });

    let shader = wgpu::ShaderModuleDescriptor {
        label: Some("Animated Model Shader"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("../../../resources/shaders/model/animated_model.wgsl").into(),
        ),
    };

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Animated Model Pipeline Layout"),
        bind_group_layouts: &[
            Some(&shared.texture),
            Some(&shared.camera),
            Some(&bones_layout),
        ],
        immediate_size: 0,
    });

    let pipeline = create_render_pipeline(
        device,
        &pipeline_layout,
        color_format,
        Some(depth_format),
        &[Vertex::desc(), InstanceUniform::desc()],
        shader,
        Some("Animated Model Pipeline"),
        Some(wgpu::CompareFunction::Less),
        Some(wgpu::Face::Back),
    );

    let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("animated_instance_buffer"),
        size: MAX_ANIMATED_DRAWS * size_of::<InstanceUniform>() as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
        mapped_at_creation: false,
    });

    AnimatedModelResources {
        pipeline,
        instance_buffer,
        bones_layout,
        bones_buffer,
        bones_bind_group,
    }
}
