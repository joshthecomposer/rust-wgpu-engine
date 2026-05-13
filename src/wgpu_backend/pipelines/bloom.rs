use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

const MAX_BLOOM_MIPS: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BloomParams {
    texel_size: [f32; 2],
    _pad0: [f32; 2],
}

struct BloomMip {
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}

pub struct BloomResources {
    _texture: wgpu::Texture,
    _sampler: wgpu::Sampler,
    params_buffer: wgpu::Buffer,
    _bind_group_layout: wgpu::BindGroupLayout,
    source_bind_group: wgpu::BindGroup,
    mip_bind_groups: Vec<wgpu::BindGroup>,
    mips: Vec<BloomMip>,
    source_width: u32,
    source_height: u32,
    downsample_pipeline: wgpu::RenderPipeline,
    upsample_pipeline: wgpu::RenderPipeline,
}

impl BloomResources {
    pub fn output_view(&self) -> &wgpu::TextureView {
        &self.mips[0].view
    }

    pub fn render<'a>(
        &'a self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
    ) -> &'a wgpu::TextureView {
        for mip_index in 0..self.mips.len() {
            let (source_width, source_height) = if mip_index == 0 {
                (self.source_width, self.source_height)
            } else {
                (
                    self.mips[mip_index - 1].width,
                    self.mips[mip_index - 1].height,
                )
            };

            self.write_params(queue, source_width, source_height);

            let bind_group = if mip_index == 0 {
                &self.source_bind_group
            } else {
                &self.mip_bind_groups[mip_index - 1]
            };

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_downsample"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.mips[mip_index].view,
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
            pass.set_pipeline(&self.downsample_pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        for mip_index in (1..self.mips.len()).rev() {
            self.write_params(
                queue,
                self.mips[mip_index].width,
                self.mips[mip_index].height,
            );

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_upsample"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.mips[mip_index - 1].view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.upsample_pipeline);
            pass.set_bind_group(0, &self.mip_bind_groups[mip_index], &[]);
            pass.draw(0..3, 0..1);
        }

        self.output_view()
    }

    fn write_params(&self, queue: &wgpu::Queue, source_width: u32, source_height: u32) {
        let params = BloomParams {
            texel_size: [
                1.0 / source_width.max(1) as f32,
                1.0 / source_height.max(1) as f32,
            ],
            _pad0: [0.0; 2],
        };
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));
    }
}

pub fn build(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    source_view: &wgpu::TextureView,
) -> BloomResources {
    let base_width = (width / 2).max(1);
    let base_height = (height / 2).max(1);
    let mip_count = mip_count(base_width, base_height);

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bloom_mip_chain"),
        size: wgpu::Extent3d {
            width: base_width,
            height: base_height,
            depth_or_array_layers: 1,
        },
        mip_level_count: mip_count,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bloom_sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    });

    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bloom_params"),
        contents: bytemuck::bytes_of(&BloomParams {
            texel_size: [1.0 / width.max(1) as f32, 1.0 / height.max(1) as f32],
            _pad0: [0.0; 2],
        }),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bloom_bind_group_layout"),
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
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let mips = (0..mip_count)
        .map(|level| BloomMip {
            view: texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("bloom_mip_view"),
                base_mip_level: level,
                mip_level_count: Some(1),
                ..Default::default()
            }),
            width: mip_size(base_width, level),
            height: mip_size(base_height, level),
        })
        .collect::<Vec<_>>();

    let source_bind_group = make_bind_group(
        device,
        &bind_group_layout,
        source_view,
        &sampler,
        &params_buffer,
        Some("bloom_source_bind_group"),
    );

    let mip_bind_groups = mips
        .iter()
        .map(|mip| {
            make_bind_group(
                device,
                &bind_group_layout,
                &mip.view,
                &sampler,
                &params_buffer,
                Some("bloom_mip_bind_group"),
            )
        })
        .collect();

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bloom_shader"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("../../../resources/shaders/post/bloom.wgsl").into(),
        ),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bloom_pipeline_layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });

    let downsample_pipeline =
        create_bloom_pipeline(device, &pipeline_layout, &shader, "fs_downsample", format, None);
    let upsample_pipeline = create_bloom_pipeline(
        device,
        &pipeline_layout,
        &shader,
        "fs_upsample",
        format,
        Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::REPLACE,
        }),
    );

    BloomResources {
        _texture: texture,
        _sampler: sampler,
        params_buffer,
        _bind_group_layout: bind_group_layout,
        source_bind_group,
        mip_bind_groups,
        mips,
        source_width: width.max(1),
        source_height: height.max(1),
        downsample_pipeline,
        upsample_pipeline,
    }
}

fn make_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    params_buffer: &wgpu::Buffer,
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
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    })
}

fn create_bloom_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    fragment_entry: &str,
    format: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(fragment_entry),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment_entry),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend,
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
    })
}

fn mip_count(width: u32, height: u32) -> u32 {
    let mut count = 1;
    let mut w = width;
    let mut h = height;

    while count < MAX_BLOOM_MIPS && (w > 1 || h > 1) {
        w = (w / 2).max(1);
        h = (h / 2).max(1);
        count += 1;
    }

    count
}

fn mip_size(base: u32, level: u32) -> u32 {
    (base >> level).max(1)
}
