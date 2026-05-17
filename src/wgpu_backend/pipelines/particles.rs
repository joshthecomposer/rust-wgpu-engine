use std::{collections::HashMap, mem::size_of};

use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::{
    assets,
    camera::Camera,
    particles::{Emitter, ParticleSystem},
    wgpu_backend::{pipelines::shared::SharedLayouts, texture::Texture},
};

const MAX_PARTICLE_INSTANCES: u64 = 200_000;

pub const FLAG_HAS_TEX: u32 = 1 << 0;
pub const FLAG_TEX_HAS_ALPHA: u32 = 1 << 1;
pub const FLAG_HAS_BLOOM: u32 = 1 << 2;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ParticleInstance {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub alpha: f32,
    pub flags: u32,
    pub _pad: [u32; 2],
}

impl ParticleInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 20]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: (size_of::<[f32; 20]>() + size_of::<f32>()) as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct QuadVertex {
    pos: [f32; 3],
    uv: [f32; 2],
}

impl QuadVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub struct ParticleResources {
    pub pipeline: wgpu::RenderPipeline,
    pub quad_vb: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub textures: HashMap<String, wgpu::BindGroup>,
    pub default_tex_bg: wgpu::BindGroup,
}

impl ParticleResources {
    /// Call BEFORE `encoder.begin_render_pass(...)`. Ensures every emitter's
    /// texture is uploaded and has a cached bind group.
    pub fn upload_pending_textures(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        tex_layout: &wgpu::BindGroupLayout,
        ps: &ParticleSystem,
    ) {
        for emitter in ps.iter_drawable() {
            let Some(path) = emitter.texture_path.as_ref() else {
                continue;
            };
            if self.textures.contains_key(path) {
                continue;
            }

            let img = match assets::load_image(path) {
                Ok(img) => img,
                Err(err) => {
                    eprintln!("particle texture load failed ({path}): {err}");
                    continue;
                }
            };
            let tex = Texture::from_image(device, queue, &img, Some(path));
            let bg = make_tex_bg(device, tex_layout, &tex);
            self.textures.insert(path.to_string(), bg);
        }
    }

    pub fn draw_all(
        &self,
        rp: &mut wgpu::RenderPass,
        queue: &wgpu::Queue,
        ps: &ParticleSystem,
        camera: &Camera,
    ) {
        rp.set_pipeline(&self.pipeline);
        rp.set_vertex_buffer(0, self.quad_vb.slice(..));

        let stride = size_of::<ParticleInstance>() as wgpu::BufferAddress;
        let mut offset: wgpu::BufferAddress = 0;

        for emitter in ps.iter_drawable() {
            let instances = build_emitter_instances(emitter, camera);
            if instances.is_empty() {
                continue;
            }

            let bytes = bytemuck::cast_slice(&instances);
            let batch_bytes = bytes.len() as wgpu::BufferAddress;

            debug_assert!(
                offset + batch_bytes <= self.instance_buffer.size(),
                "particle instance_buffer too small"
            );

            queue.write_buffer(&self.instance_buffer, offset, bytes);

            let bg = emitter
                .texture_path
                .as_ref()
                .and_then(|p| self.textures.get(p))
                .unwrap_or(&self.default_tex_bg);

            rp.set_bind_group(0, bg, &[]);
            rp.set_vertex_buffer(1, self.instance_buffer.slice(offset..offset + batch_bytes));
            rp.draw(0..6, 0..instances.len() as u32);

            offset += instances.len() as wgpu::BufferAddress * stride;
        }
    }
}

fn build_emitter_instances(emitter: &Emitter, camera: &Camera) -> Vec<ParticleInstance> {
    if emitter.count == 0 {
        return Vec::new();
    }

    // View-aligned billboard: bake inverse view rotation into the model.
    let view = camera.view;
    let view_rot = Mat3::from_cols(
        view.x_axis.truncate(),
        view.y_axis.truncate(),
        view.z_axis.truncate(),
    );
    let inv_view_rot = view_rot.transpose();
    let model_rot = Mat4::from_mat3(inv_view_rot);

    let mut flags: u32 = 0;
    if emitter.texture_path.is_some() {
        flags |= FLAG_HAS_TEX;
    }
    if emitter.texture_has_alpha {
        flags |= FLAG_TEX_HAS_ALPHA;
    }
    if emitter.has_bloom {
        flags |= FLAG_HAS_BLOOM;
    }

    let mut out = Vec::with_capacity(emitter.count);

    for i in 0..emitter.count {
        let t = emitter.times_alive[i];
        let life = emitter.lifetimes[i].max(1e-5);
        let t_norm = (t / life).clamp(0.0, 1.0);

        let alpha_t = t_norm.powf(emitter.alpha_powers[i]);
        let start_alpha = emitter.base_alphas[i];
        let end_alpha = emitter.end_alphas[i];
        let a = (start_alpha + (end_alpha - start_alpha) * alpha_t).clamp(0.0, 1.0);

        let scale_t = t_norm.powf(emitter.scale_powers[i]);
        let start_factor = emitter.base_scales[i];
        let end_factor = emitter.end_scales[i];
        let factor = start_factor + (end_factor - start_factor) * scale_t;
        let scale = emitter.scales[i] * factor;

        let model = Mat4::from_translation(emitter.positions[i])
            * model_rot
            * Mat4::from_scale(Vec3::splat(scale));

        out.push(ParticleInstance {
            model: model.to_cols_array_2d(),
            color: emitter.colors[i].to_array(),
            alpha: a,
            flags,
            _pad: [0; 2],
        });
    }

    out
}

fn make_tex_bg(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    tex: &Texture,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("particle_tex_bg"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&tex.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&tex.sampler),
            },
        ],
    })
}

pub fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    shared: &SharedLayouts,
    scene_format: wgpu::TextureFormat,
    bright_format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
) -> ParticleResources {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Particles Shader"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("../../../resources/shaders/particles.wgsl").into(),
        ),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Particles Pipeline Layout"),
        bind_group_layouts: &[Some(&shared.texture), Some(&shared.camera)],
        immediate_size: 0,
    });

    // Particle-specific MRT blend:
    //   scene  = SrcAlpha / (1 - SrcAlpha)  -> standard alpha compositing
    //   bright = One / One                  -> additive emissive accumulation
    // Built inline (not via the shared helper) because particles need
    // depth_write_enabled=false and per-target blend states.
    let particle_color_targets = [
        Some(wgpu::ColorTargetState {
            format: scene_format,
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
        }),
        Some(wgpu::ColorTargetState {
            format: bright_format,
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::REPLACE,
            }),
            write_mask: wgpu::ColorWrites::ALL,
        }),
    ];

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Particles Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[QuadVertex::desc(), ParticleInstance::desc()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &particle_color_targets,
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
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

    // Two triangles forming a [-1, 1]^2 quad with UVs.
    // Matches the GL setup so existing textures map identically.
    let quad: [QuadVertex; 6] = [
        QuadVertex {
            pos: [-1.0, 1.0, 0.0],
            uv: [0.0, 1.0],
        },
        QuadVertex {
            pos: [-1.0, -1.0, 0.0],
            uv: [0.0, 0.0],
        },
        QuadVertex {
            pos: [1.0, -1.0, 0.0],
            uv: [1.0, 0.0],
        },
        QuadVertex {
            pos: [-1.0, 1.0, 0.0],
            uv: [0.0, 1.0],
        },
        QuadVertex {
            pos: [1.0, -1.0, 0.0],
            uv: [1.0, 0.0],
        },
        QuadVertex {
            pos: [1.0, 1.0, 0.0],
            uv: [1.0, 1.0],
        },
    ];

    let quad_vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("particle_quad_vb"),
        contents: bytemuck::cast_slice(&quad),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("particle_instance_buffer"),
        size: MAX_PARTICLE_INSTANCES * size_of::<ParticleInstance>() as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
        mapped_at_creation: false,
    });

    let default_tex = Texture::from_solid_rgba8_srgb(
        device,
        queue,
        [255, 255, 255, 255],
        Some("particle_default_tex"),
    );
    let default_tex_bg = make_tex_bg(device, &shared.texture, &default_tex);

    ParticleResources {
        pipeline,
        quad_vb,
        instance_buffer,
        textures: HashMap::new(),
        default_tex_bg,
    }
}
