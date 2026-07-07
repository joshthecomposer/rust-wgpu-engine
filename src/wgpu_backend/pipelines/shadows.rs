use wgpu::util::DeviceExt;

use crate::{
    camera::Camera,
    enums_types::InstanceUniform,
    lights::Lights,
    util::constants::{SHADOW_HEIGHT, SHADOW_WIDTH},
    wgpu_backend::{
        pipelines::{create_depth_only_pipeline, shared::SharedLayouts},
        vertex::Vertex,
    },
};

pub const SHADOW_DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightSpaceUniform {
    pub light_view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowSampleUniform {
    pub bias_scalar: f32,
    pub border_fallback: u32,
    pub _pad: [u32; 2],
}

pub struct ShadowResources {
    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
    pub depth_sampler: wgpu::Sampler,
    pub light_buffer: wgpu::Buffer,
    pub light_bind_group: wgpu::BindGroup,
    pub sample_buffer: wgpu::Buffer,
    /// Directional light + shadow map (group 2 static, group 3 animated).
    pub lighting_bind_group: wgpu::BindGroup,
    pub static_pipeline: wgpu::RenderPipeline,
    pub animated_pipeline: wgpu::RenderPipeline,
}

impl ShadowResources {
    pub fn build(
        device: &wgpu::Device,
        shared: &SharedLayouts,
        dir_light_buffer: &wgpu::Buffer,
        bones_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shadow_size = wgpu::Extent3d {
            width: SHADOW_WIDTH as u32,
            height: SHADOW_HEIGHT as u32,
            depth_or_array_layers: 1,
        };

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow_depth_texture"),
            size: shadow_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SHADOW_DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let depth_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow_depth_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            compare: None,
            ..Default::default()
        });

        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("light_space_buffer"),
            contents: bytemuck::bytes_of(&LightSpaceUniform {
                light_view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("light_space_bind_group"),
            layout: &shared.light_space,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
        });

        let sample_uniform = ShadowSampleUniform {
            bias_scalar: 0.002,
            border_fallback: 0,
            _pad: [0, 0],
        };

        let sample_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow_sample_buffer"),
            contents: bytemuck::bytes_of(&sample_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let lighting_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lighting_bind_group"),
            layout: &shared.dir_light,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: dir_light_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&depth_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: sample_buffer.as_entire_binding(),
                },
            ],
        });

        let static_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("static_shadow_pipeline_layout"),
            bind_group_layouts: &[Some(&shared.light_space)],
            immediate_size: 0,
        });

        let static_pipeline = create_depth_only_pipeline(
            device,
            &static_layout,
            SHADOW_DEPTH_FORMAT,
            &[Vertex::desc(), InstanceUniform::desc()],
            wgpu::ShaderModuleDescriptor {
                label: Some("static_shadow_depth"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../../resources/shaders/shadow/static_shadow_depth.wgsl")
                        .into(),
                ),
            },
            Some("static_shadow_pipeline"),
            Some(wgpu::Face::Front),
        );

        let animated_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("animated_shadow_pipeline_layout"),
            bind_group_layouts: &[Some(&shared.light_space), Some(bones_layout)],
            immediate_size: 0,
        });

        let animated_pipeline = create_depth_only_pipeline(
            device,
            &animated_layout,
            SHADOW_DEPTH_FORMAT,
            &[Vertex::desc(), InstanceUniform::desc()],
            wgpu::ShaderModuleDescriptor {
                label: Some("animated_shadow_depth"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../../resources/shaders/shadow/animated_shadow_depth.wgsl")
                        .into(),
                ),
            },
            Some("animated_shadow_pipeline"),
            Some(wgpu::Face::Front),
        );

        Self {
            depth_texture,
            depth_view,
            depth_sampler,
            light_buffer,
            light_bind_group,
            sample_buffer,
            lighting_bind_group,
            static_pipeline,
            animated_pipeline,
        }
    }

    pub fn compute_light_space(lights: &Lights, camera: &Camera, alpha: f32) -> glam::Mat4 {
        let near_plane = lights.near;
        let far_plane = lights.far;
        let half_bound = lights.bounds;

        let light_dir = lights.dir_light.direction.normalize();
        let light_distance = lights.dir_light.distance;

        // Match the interpolated view / instance transforms used during the scene pass.
        let position = camera.prev_pos.lerp(camera.position, alpha);
        let forward = camera.prev_forward.lerp(camera.forward, alpha).normalize();

        let shadow_push = half_bound * 1.2;
        let shadow_center = position + forward * shadow_push;
        let light_pos = shadow_center + light_dir * light_distance;

        // Vulkan-style depth [0, 1]; shaders remap XY only (see shadow_calculation in model WGSL).
        let light_projection = glam::Mat4::orthographic_rh(
            -half_bound,
            half_bound,
            -half_bound,
            half_bound,
            near_plane,
            far_plane,
        );

        let light_view =
            glam::Mat4::look_at_rh(light_pos, shadow_center, glam::Vec3::new(0.0, 1.0, 0.0));

        light_projection * light_view
    }

    pub fn write_light_space(&self, queue: &wgpu::Queue, light_space: glam::Mat4) {
        queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::bytes_of(&LightSpaceUniform {
                light_view_proj: light_space.to_cols_array_2d(),
            }),
        );
    }

    pub fn write_sample_uniforms(&self, queue: &wgpu::Queue, lights: &Lights) {
        let border_fallback = 0u32;

        queue.write_buffer(
            &self.sample_buffer,
            0,
            bytemuck::bytes_of(&ShadowSampleUniform {
                bias_scalar: lights.bias_scalar,
                border_fallback,
                _pad: [0, 0],
            }),
        );
    }
}
