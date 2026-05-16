use std::num::NonZeroU64;

use wgpu::util::DeviceExt;

use crate::wgpu_backend::texture;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct HdrCompositeParams {
    pub exposure: f32,
    pub hdr_enabled: u32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub inv_proj: [[f32; 4]; 4],
}

impl HdrCompositeParams {
    pub fn new() -> Self {
        Self {
            exposure: 1.0,
            hdr_enabled: 1,
            _pad0: 0,
            _pad1: 0,
            inv_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}

pub struct HdrResources {
    pub scene_color: texture::Texture, // Rgba16Float
    pub scene_format: wgpu::TextureFormat,

    pub width: u32,
    pub height: u32,

    // fullscreen composite (tonemap, gamma, fog, etc)
    pub composite_layout: wgpu::BindGroupLayout,
    pub composite_bind_group: wgpu::BindGroup,
    pub composite_pipeline: wgpu::RenderPipeline,

    pub composite_uniforms: wgpu::Buffer,
}

impl HdrResources {
    pub fn scene_view(&self) -> &wgpu::TextureView {
        &self.scene_color.view
    }

    pub fn composite_pass(&self, encoder: &mut wgpu::CommandEncoder, output: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("hdr_composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.composite_pipeline);
        pass.set_bind_group(0, &self.composite_bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    pub fn write_params(&self, queue: &wgpu::Queue, params: HdrCompositeParams) {
        queue.write_buffer(&self.composite_uniforms, 0, bytemuck::bytes_of(&params));
    }
}

fn scene_hdr_format() -> wgpu::TextureFormat {
    wgpu::TextureFormat::Rgba16Float
}

fn create_scene_hdr_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
) -> texture::Texture {
    let format = scene_hdr_format();
    let size = wgpu::Extent3d {
        width: width.max(1),
        height: height.max(1),
        depth_or_array_layers: 1,
    };
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hdr_scene_color"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("hdr_scene_sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    });

    texture::Texture {
        texture: tex,
        view,
        sampler,
    }
}
pub fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    surface_format: wgpu::TextureFormat, // composite output = swapchain format
    initial_params: HdrCompositeParams,
    depth_view: &wgpu::TextureView,
) -> HdrResources {
    let scene_format = scene_hdr_format();
    let scene_color = create_scene_hdr_texture(device, queue, width, height);
    let ub_size = std::mem::size_of::<HdrCompositeParams>() as u64;

    let composite_uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("hdr_composite_uniforms"),
        contents: bytemuck::bytes_of(&initial_params),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let composite_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("hdr_composite_layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(ub_size),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
        ],
    });

    let composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("hdr_composite_bind_group"),
        layout: &composite_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&scene_color.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&scene_color.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: composite_uniforms.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(depth_view),
            },
        ],
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("hdr_composite_shader"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("../../../resources/shaders/post/hdr_composite.wgsl").into(),
        ),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("hdr_composite_pipeline_layout"),
        bind_group_layouts: &[Some(&composite_layout)],
        immediate_size: 0,
    });
    // composite: no vertex buffers (fullscreen triangle in VS)
    let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("hdr_composite_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });
    HdrResources {
        scene_color,
        scene_format,
        width,
        height,
        composite_layout,
        composite_bind_group,
        composite_pipeline,
        composite_uniforms,
    }
}
