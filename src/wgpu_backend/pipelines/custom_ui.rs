use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::ui::game_new::render::UiVertex;

/// Uniform pushed once per frame: pixel-space screen size for the vertex
/// shader's NDC conversion.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct UiUniforms {
    pub screen_size: [f32; 2],
    pub _pad: [f32; 2],
}

/// Owns the two `RenderPipeline`s and the layouts they share.
///
/// `solid` outputs `v_color * tex.rgba` (used by every textured/colored rect
/// — a 1x1 white texture covers flat fills).
/// `alpha_mask` outputs `v_color * vec4(1, 1, 1, tex.r)` (used by the glyph
/// atlas, which is `R8Unorm`).
///
/// Both pipelines share `screen_layout` (group 0, the shared uniform) and
/// `texture_layout` (group 1, swapped per-draw by the texture registry).
pub struct UiPipelines {
    pub solid: wgpu::RenderPipeline,
    pub alpha_mask: wgpu::RenderPipeline,

    pub screen_layout: wgpu::BindGroupLayout,
    pub texture_layout: wgpu::BindGroupLayout,

    pub screen_uniform_buffer: wgpu::Buffer,
    pub screen_bind_group: wgpu::BindGroup,
}

pub fn build(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> UiPipelines {
    let screen_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("ui_screen_bind_group_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("ui_texture_bind_group_layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let screen_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ui_screen_uniform_buffer"),
        contents: bytemuck::bytes_of(&UiUniforms {
            screen_size: [1.0, 1.0],
            _pad: [0.0, 0.0],
        }),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let screen_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ui_screen_bind_group"),
        layout: &screen_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: screen_uniform_buffer.as_entire_binding(),
        }],
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("custom_ui_shader"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("../../../resources/shaders/ui/custom_ui.wgsl").into(),
        ),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("custom_ui_pipeline_layout"),
        bind_group_layouts: &[Some(&screen_layout), Some(&texture_layout)],
        immediate_size: 0,
    });

    let solid = create_ui_pipeline(
        device,
        &pipeline_layout,
        &shader,
        "fs_solid",
        surface_format,
        "custom_ui_solid_pipeline",
    );
    let alpha_mask = create_ui_pipeline(
        device,
        &pipeline_layout,
        &shader,
        "fs_alpha_mask",
        surface_format,
        "custom_ui_alpha_mask_pipeline",
    );

    UiPipelines {
        solid,
        alpha_mask,
        screen_layout,
        texture_layout,
        screen_uniform_buffer,
        screen_bind_group,
    }
}

/// Build a per-texture bind group bound to group 1 of either pipeline.
///
/// Called by the texture registry every time a new `texture_id` is minted
/// (1x1 white pixel at id 0, glyph atlas at id 1, then any user textures).
pub fn make_texture_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    label: Option<&str>,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label,
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

fn create_ui_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    fs_entry: &str,
    surface_format: wgpu::TextureFormat,
    label: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[UiVertex::desc()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs_entry),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}
