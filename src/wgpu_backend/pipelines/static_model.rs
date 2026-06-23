use std::mem::size_of;

use crate::{
    entity_manager::EntityManager,
    enums_types::{InstanceUniform, Transform},
    wgpu_backend::{
        model::{DrawDepthOnly, DrawModel, Model},
        pipelines::{
            create_render_pipeline,
            shared::{self, SharedLayouts},
        },
        vertex::Vertex,
    },
};

const MAX_STATIC_INSTANCES: u64 = 10_000;

pub struct StaticBatch<'a> {
    pub model: &'a Model,
    pub byte_offset: wgpu::BufferAddress,
    pub instance_count: u32,
}

pub struct StaticModelResources {
    pub pipeline: wgpu::RenderPipeline,
    pub instance_buffer: wgpu::Buffer,
}

impl StaticModelResources {
    pub fn prepare<'a>(
        &self,
        queue: &wgpu::Queue,
        em: &'a EntityManager,
        alpha: f32,
        scratch: &mut Vec<InstanceUniform>,
        out: &mut Vec<StaticBatch<'a>>,
    ) {
        scratch.clear();
        out.clear();

        let stride = size_of::<InstanceUniform>() as wgpu::BufferAddress;
        let mut offset: wgpu::BufferAddress = 0;

        for ids in em.get_modeled_static_ids_by_type().values() {
            if ids.is_empty() {
                continue;
            }

            let batch_start = scratch.len();
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

                scratch.push(Transform::interpolated(prev, curr, alpha).to_instance_uniform());
            }

            let Some(model) = batch_model else { continue };
            let count = scratch.len() - batch_start;
            if count == 0 {
                continue;
            }

            let bytes = bytemuck::cast_slice(&scratch[batch_start..]);
            let batch_bytes = bytes.len() as wgpu::BufferAddress;

            debug_assert!(
                offset + batch_bytes <= self.instance_buffer.size(),
                "instance_buffer too small"
            );

            queue.write_buffer(&self.instance_buffer, offset, bytes);

            out.push(StaticBatch {
                model,
                byte_offset: offset,
                instance_count: count as u32,
            });

            offset += count as wgpu::BufferAddress * stride;
        }
    }

    pub fn draw_prepared<'a>(
        &self,
        rp: &mut wgpu::RenderPass<'_>,
        pipeline: &wgpu::RenderPipeline,
        batches: &[StaticBatch<'a>],
        lit_pass: bool,
    ) {
        let stride = size_of::<InstanceUniform>() as wgpu::BufferAddress;
        rp.set_pipeline(pipeline);

        for batch in batches {
            let end = batch.byte_offset + batch.instance_count as wgpu::BufferAddress * stride;
            rp.set_vertex_buffer(1, self.instance_buffer.slice(batch.byte_offset..end));

            if lit_pass {
                rp.draw_model_instanced(batch.model, 0..batch.instance_count);
            } else {
                rp.draw_model_depth_only(batch.model, 0..batch.instance_count);
            }
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
