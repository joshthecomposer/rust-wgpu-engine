use std::{mem::size_of, num::NonZeroU64};

use crate::{
    entity_manager::EntityManager,
    enums_types::{InstanceUniform, Transform},
    wgpu_backend::{
        bone_uniforms::BoneUniforms,
        model::{DrawDepthOnly, DrawModel},
        pipelines::{
            create_render_pipeline,
            shared::{self, SharedLayouts},
        },
        vertex::Vertex,
    },
};

const MAX_ANIMATED_DRAWS: u64 = 256;

pub struct AnimatedBatch<'a> {
    pub model: &'a crate::wgpu_backend::model::Model,
    pub instance_offset: wgpu::BufferAddress,
    pub bones_offset: wgpu::BufferAddress,
}

pub struct AnimatedModelResources {
    pub pipeline: wgpu::RenderPipeline,
    pub instance_buffer: wgpu::Buffer,
    pub bones_layout: wgpu::BindGroupLayout,
    pub bones_buffer: wgpu::Buffer,
    pub bones_bind_group: wgpu::BindGroup,
}

impl AnimatedModelResources {
    pub fn prepare<'a>(
        &self,
        queue: &wgpu::Queue,
        em: &'a EntityManager,
        alignment: usize,
        alpha: f32,
        out: &mut Vec<AnimatedBatch<'a>>,
    ) {
        out.clear();

        let instance_stride = size_of::<InstanceUniform>() as wgpu::BufferAddress;
        let bone_stride = (size_of::<BoneUniforms>() as wgpu::BufferAddress)
            .next_multiple_of(alignment as wgpu::BufferAddress);

        let mut instance_byte_offset: wgpu::BufferAddress = 0;
        let mut bones_byte_offset: wgpu::BufferAddress = 0;

        for ids in em.get_animated_ids_by_type().values() {
            for id in ids {
                let model = em.models.get(*id).unwrap();
                let animator = em.animators.get(*id).unwrap();
                let anim = animator.get_current_animation().unwrap();
                let curr = em.transforms.get(*id).unwrap();
                let prev = em.prev_transforms.get(*id).unwrap();

                let instance = Transform::interpolated(prev, curr, alpha).to_instance_uniform();

                queue.write_buffer(
                    &self.instance_buffer,
                    instance_byte_offset,
                    bytemuck::cast_slice(&[instance]),
                );

                queue.write_buffer(
                    &self.bones_buffer,
                    bones_byte_offset,
                    bytemuck::cast_slice(&anim.current_pose),
                );

                out.push(AnimatedBatch {
                    model,
                    instance_offset: instance_byte_offset,
                    bones_offset: bones_byte_offset,
                });

                instance_byte_offset += instance_stride;
                bones_byte_offset += bone_stride;
            }
        }
    }

    pub fn draw_prepared<'a>(
        &self,
        rp: &mut wgpu::RenderPass<'_>,
        pipeline: &wgpu::RenderPipeline,
        batches: &[AnimatedBatch<'a>],
        lit_pass: bool,
    ) {
        let instance_stride = size_of::<InstanceUniform>() as wgpu::BufferAddress;
        rp.set_pipeline(pipeline);

        for batch in batches {
            rp.set_vertex_buffer(
                1,
                self.instance_buffer.slice(
                    batch.instance_offset..batch.instance_offset + instance_stride,
                ),
            );

            let bones_dynamic_offset: wgpu::DynamicOffset = batch
                .bones_offset
                .try_into()
                .expect("bones slab offset fits u32");

            if lit_pass {
                rp.draw_model_animated(
                    batch.model,
                    &self.bones_bind_group,
                    bones_dynamic_offset,
                );
            } else {
                rp.draw_model_depth_only_animated(
                    batch.model,
                    &self.bones_bind_group,
                    bones_dynamic_offset,
                    1,
                );
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

    let shader_wgsl: &str =
        include_str!("../../../resources/shaders/model/animated_model.wgsl");

    let shader = wgpu::ShaderModuleDescriptor {
        label: Some("Animated Model Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_wgsl.into()),
    };

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Animated Model Pipeline Layout"),
        bind_group_layouts: &[
            Some(&shared.texture),
            Some(&shared.camera),
            Some(&bones_layout),
            Some(&shared.dir_light),
        ],
        immediate_size: 0,
    });

    let scene_targets = shared::scene_color_targets(scene_format, bright_format);

    let pipeline = create_render_pipeline(
        device,
        &pipeline_layout,
        &scene_targets,
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
