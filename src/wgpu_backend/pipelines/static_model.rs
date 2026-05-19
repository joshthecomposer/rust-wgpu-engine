use std::mem::size_of;

use crate::{
    entity_manager::EntityManager,
    enums_types::{InstanceUniform, Transform},
    wgpu_backend::{
        model::{DrawModel, Model},
        pipelines::{
            create_render_pipeline,
            shared::{self, SharedLayouts},
        },
        vertex::Vertex,
    },
};

const MAX_STATIC_INSTANCES: u64 = 10_000;

pub struct StaticModelResources {
    pub pipeline: wgpu::RenderPipeline,
    pub instance_buffer: wgpu::Buffer,
}

impl StaticModelResources {
    pub fn draw_all(
        &self,
        rp: &mut wgpu::RenderPass,
        queue: &wgpu::Queue,
        em: &EntityManager,
        alpha: f32,
    ) {
        let stride = std::mem::size_of::<InstanceUniform>() as wgpu::BufferAddress;
        let mut offset: wgpu::BufferAddress = 0;

        rp.set_pipeline(&self.pipeline);

        for ids in em.get_modeled_static_ids_by_type().values() {
            if ids.is_empty() {
                continue;
            }

            let mut instances = Vec::new();
            let mut batch_model: Option<&Model> = None;

            for &id in ids.iter() {
                let (Some(curr), Some(prev), Some(model)) = (
                    em.transforms.get(id),
                    em.prev_transforms.get(id),
                    em.models.get(id),
                ) else {
                    continue;
                };

                if batch_model.is_none() {
                    batch_model = Some(model);
                }

                instances.push(Transform::interpolated(prev, curr, alpha).to_instance_uniform());
            }

            let Some(model) = batch_model else { continue };

            if instances.is_empty() {
                continue;
            }

            let bytes = bytemuck::cast_slice(&instances);

            let batch_bytes = bytes.len() as wgpu::BufferAddress;

            debug_assert!(
                offset + batch_bytes <= self.instance_buffer.size(),
                "instance_buffer too small"
            );

            queue.write_buffer(&self.instance_buffer, offset, bytes);

            rp.set_vertex_buffer(1, self.instance_buffer.slice(offset..offset + batch_bytes));

            rp.draw_model_instanced(model, 0..instances.len() as u32);

            offset += instances.len() as wgpu::BufferAddress * stride;
        }
    }
}

pub fn build(
    device: &wgpu::Device,
    shared: &SharedLayouts,
    scene_format: wgpu::TextureFormat,
    bright_format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
    #[cfg(target_arch = "wasm32")] depth_proxy_format: wgpu::TextureFormat,
) -> StaticModelResources {
    #[cfg(not(target_arch = "wasm32"))]
    let shader_wgsl: &str =
        include_str!("../../../resources/shaders/model/static_model.wgsl");
    #[cfg(target_arch = "wasm32")]
    let shader_wgsl: &str =
        include_str!("../../../resources/shaders/model/static_model_wasm.wgsl");

    let shader = wgpu::ShaderModuleDescriptor {
        label: Some("Static Model Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_wgsl.into()),
    };

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Static Model Pipeline Layout"),
        bind_group_layouts: &[
            Some(&shared.texture),
            Some(&shared.camera),
            Some(&shared.dir_light),
        ],
        immediate_size: 0,
    });

    #[cfg(not(target_arch = "wasm32"))]
    let scene_targets = shared::scene_color_targets(scene_format, bright_format);
    #[cfg(target_arch = "wasm32")]
    let scene_targets =
        shared::scene_color_targets_wasm(scene_format, bright_format, depth_proxy_format);

    let pipeline = create_render_pipeline(
        device,
        &pipeline_layout,
        &scene_targets,
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
