use crate::{
    enums_types::InstanceUniform,
    wgpu_backend::{pipelines::shared::SharedLayouts, vertex::Vertex},
};

pub struct GizmoPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

pub fn build(
    device: &wgpu::Device,
    shared: &SharedLayouts,
    scene_format: wgpu::TextureFormat,
    bright_format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
) -> GizmoPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Gizmo Shader"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("../../../resources/shaders/gizmo.wgsl").into(),
        ),
    });

    // Single contiguous bind group: camera at slot 0. Gizmos don't use any
    // textures or lights, so we don't mirror the scene shaders' [tex, cam, light]
    // shape — keeping the layout small avoids any compatibility surprises.
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Gizmo Pipeline Layout"),
        bind_group_layouts: &[Some(&shared.camera)],
        immediate_size: 0,
    });

    // Lines must not write depth so they don't occlude later draws (e.g. the
    // bloom/HDR composite path); depth_compare=Less still gives correct
    // occlusion by world geometry already in the depth buffer.
    let color_targets = [
        Some(wgpu::ColorTargetState {
            format: scene_format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        }),
        Some(wgpu::ColorTargetState {
            format: bright_format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        }),
    ];

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Gizmo Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Vertex::desc(), InstanceUniform::desc()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &color_targets,
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: depth_format,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
    });

    GizmoPipeline { pipeline }
}
