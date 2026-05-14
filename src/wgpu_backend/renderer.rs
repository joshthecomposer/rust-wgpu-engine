use std::{collections::HashMap, mem::size_of, num::NonZeroU64, sync::Arc};
use winit::window::Window;

use wgpu::{util::DeviceExt, RenderPipeline};

use crate::{
    camera::{Camera, CameraUniform},
    entity_manager::EntityManager,
    enums_types::InstanceUniform,
    util::constants::MAX_BONES,
    wgpu_backend::{
        bind_group_layout_type::BindGroupLayoutType, bone_uniforms::BoneUniforms, model::Model,
        pipeline_type::PipelineType, texture, vertex::Vertex,
    },
};

/// Max skinned meshes drawn per frame; sizes instance + bones ring uploads.
/// Each slot uses disjoint buffer ranges so queued `write_buffer`s are valid before draws run.
const MAX_ANIMATED_DRAWS: u64 = 256;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct Renderer {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    pub pipelines: HashMap<PipelineType, RenderPipeline>,
    pub layouts: HashMap<BindGroupLayoutType, wgpu::BindGroupLayout>,
    pub bind_groups: HashMap<BindGroupLayoutType, wgpu::BindGroup>,
    pub camera_buffer: wgpu::Buffer,

    pub instance_buffer: wgpu::Buffer,
    /// One `InstanceUniform` per skinned draw; never overlaps static instancing uploads.
    pub animated_instance_buffer: wgpu::Buffer,
    pub bones_buffer: wgpu::Buffer,
    pub depth_texture: texture::Texture,
}

impl Renderer {
    pub async fn new(window: Arc<Window>, camera_uniform: CameraUniform) -> Self {
        let inner = window.inner_size();

        let width = inner.width.max(1);
        let height = inner.height.max(1);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .or_else(|| surface_caps.formats.first().copied())
            .unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb);

        let present_mode = surface_caps
            .present_modes
            .first()
            .copied()
            .unwrap_or(wgpu::PresentMode::Fifo);

        let alpha_mode = surface_caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let mut layouts = HashMap::new();
        let mut bind_groups = HashMap::new();
        let mut pipelines = HashMap::new();

        // =============================================
        // Main Texture Bind Group
        // =============================================
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                label: Some("Texture_bind_group_layout"),
            });

        layouts.insert(
            BindGroupLayoutType::Texture,
            texture_bind_group_layout.clone(),
        );

        // =============================================
        // Depth Texture Bind Group
        // =============================================
        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        // =============================================
        // Camera Bind Group
        // =============================================

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        layouts.insert(
            BindGroupLayoutType::Camera,
            camera_bind_group_layout.clone(),
        );
        bind_groups.insert(BindGroupLayoutType::Camera, camera_bind_group);

        // ==============================================
        // STATIC MODEL PIPELINE
        // ==============================================
        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("Static Model Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../resources/shaders/model/static_model.wgsl").into(),
            ),
        };

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Model Pipeline Layout"),
            bind_group_layouts: &[
                Some(&texture_bind_group_layout),
                Some(&camera_bind_group_layout),
            ],
            immediate_size: 0,
        });

        pipelines.insert(
            PipelineType::Model,
            create_render_pipeline(
                &device,
                &layout,
                config.format,
                Some(DEPTH_FORMAT),
                &[Vertex::desc(), InstanceUniform::desc()],
                shader,
            ),
        );

        // ==============================================
        // ANIMATED MODEL PIPELINE
        // ==============================================

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("Animated Model Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../resources/shaders/model/animated_model.wgsl").into(),
            ),
        };

        let bone_uniform_size = NonZeroU64::new(size_of::<BoneUniforms>() as u64)
            .expect("BoneUniforms must be non-empty");
        let bone_buffer_size = bone_uniform_size.get() * MAX_ANIMATED_DRAWS;
        let bone_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bone_uniform_buffer"),
            size: bone_buffer_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bones_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            layout: &bones_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &bone_buffer,
                    offset: 0,
                    size: Some(bone_uniform_size),
                }),
            }],
        });

        layouts.insert(BindGroupLayoutType::Bones, bones_bind_group_layout.clone());
        bind_groups.insert(BindGroupLayoutType::Bones, bones_bind_group);

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Animated Model Pipeline Layout"),
            bind_group_layouts: &[
                Some(&texture_bind_group_layout),
                Some(&camera_bind_group_layout),
                Some(&bones_bind_group_layout),
            ],
            immediate_size: 0,
        });

        pipelines.insert(
            PipelineType::AnimatedModel,
            create_render_pipeline(
                &device,
                &layout,
                config.format,
                Some(DEPTH_FORMAT),
                &[Vertex::desc(), InstanceUniform::desc()],
                shader,
            ),
        );

        // ==============================================
        // Instance Buffer
        // ==============================================
        let max_instances: u64 = 10_000;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("static_instance_buffer"),
            size: max_instances * size_of::<InstanceUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let anim_stride = size_of::<InstanceUniform>() as u64;
        let animated_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("animated_instance_buffer"),
            size: anim_stride * MAX_ANIMATED_DRAWS,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        Self {
            surface,
            device,
            queue,
            config,
            pipelines,
            layouts,
            bind_groups,
            camera_buffer,
            instance_buffer,
            animated_instance_buffer,
            depth_texture,
            bones_buffer: bone_buffer,
        }
    }

    pub fn render_world(&mut self, aspect: f32, camera: &Camera, em: &EntityManager) {
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera.uniform));

        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                self.surface.configure(&self.device, &self.config);
                surface_texture
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                // Skip this frame
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // You could recreate the devices and all resources
                // created with it here, but we'll just bail
                panic!("Lost device");
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render encoder"),
            });

        // ==============================================
        // Static Model Pass
        // ==============================================
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 0.2,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),

                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            let pipeline = self.pipelines.get(&PipelineType::Model).unwrap();
            let cam_bg = self.bind_groups.get(&BindGroupLayoutType::Camera).unwrap();

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(1, cam_bg, &[]);

            let ids_by_type = em.get_modeled_static_ids_by_type();

            let instance_stride = std::mem::size_of::<InstanceUniform>() as wgpu::BufferAddress;
            let mut instance_offset: wgpu::BufferAddress = 0;

            for ids in ids_by_type.values() {
                let mut instances = vec![];
                let mut batch_model: Option<&Model> = None;
                if ids.is_empty() {
                    continue;
                }

                for id in ids {
                    let id = *id;

                    let (Some(transform), Some(model)) = (em.transforms.get(id), em.models.get(id))
                    else {
                        continue;
                    };

                    if batch_model.is_none() {
                        batch_model = Some(model);
                    }

                    instances.push(transform.to_instance_uniform());
                }

                if let Some(model) = batch_model {
                    if !instances.is_empty() {
                        let instance_bytes = bytemuck::cast_slice(&instances);
                        let batch_bytes = instance_bytes.len() as wgpu::BufferAddress;

                        debug_assert!(
                            instance_offset + batch_bytes <= self.instance_buffer.size(),
                            "instance_buffer too small for this frame"
                        );

                        self.queue.write_buffer(
                            &self.instance_buffer,
                            instance_offset,
                            instance_bytes,
                        );

                        render_pass.set_bind_group(0, &model.material.bind_group, &[]);
                        render_pass.set_vertex_buffer(0, model.vertex_buffer.slice(..));
                        render_pass.set_vertex_buffer(
                            1,
                            self.instance_buffer
                                .slice(instance_offset..instance_offset + batch_bytes),
                        );

                        render_pass.set_index_buffer(
                            model.index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );

                        let n = instances.len() as u32;

                        render_pass.draw_indexed(0..model.num_elements, 0, 0..n);

                        instance_offset += instances.len() as wgpu::BufferAddress * instance_stride;
                    }
                }
            }
        }

        // ==============================================
        // Animated Model Pass
        // ==============================================

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),

                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            let pipeline = self.pipelines.get(&PipelineType::AnimatedModel).unwrap();
            let cam_bg = self.bind_groups.get(&BindGroupLayoutType::Camera).unwrap();

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(1, cam_bg, &[]);

            let ids_by_type = em.get_animated_ids_by_type();

            let anim_stride_u64 = size_of::<InstanceUniform>() as u64;
            let bone_stride = size_of::<BoneUniforms>() as wgpu::BufferAddress;
            let bones_bg = self.bind_groups.get(&BindGroupLayoutType::Bones).unwrap();

            let mut animated_slot: u64 = 0;

            for ids in ids_by_type.values() {
                if ids.is_empty() {
                    continue;
                }

                for id in ids {
                    let id = *id;

                    let (Some(transform), Some(model), Some(animator)) = (
                        em.transforms.get(id),
                        em.models.get(id),
                        em.animators.get(id),
                    ) else {
                        continue;
                    };

                    debug_assert!(
                        animated_slot < MAX_ANIMATED_DRAWS,
                        "animated draw count exceeds MAX_ANIMATED_DRAWS ({MAX_ANIMATED_DRAWS}); raise constant or batch"
                    );

                    let animation = animator.get_current_animation().unwrap();
                    let pose = &animation.current_pose;

                    let n = pose.len().min(MAX_BONES as usize);

                    let mut bones = BoneUniforms {
                        matrices: [glam::Mat4::IDENTITY; MAX_BONES as usize],
                    };

                    bones.matrices[..n].copy_from_slice(&pose[..n]);

                    let bone_bytes = bytemuck::bytes_of(&bones);

                    let inst_byte_off = animated_slot * anim_stride_u64;
                    let bone_byte_off = animated_slot as wgpu::BufferAddress * bone_stride;

                    self.queue.write_buffer(
                        &self.animated_instance_buffer,
                        inst_byte_off,
                        bytemuck::bytes_of(&transform.to_instance_uniform()),
                    );

                    self.queue
                        .write_buffer(&self.bones_buffer, bone_byte_off, bone_bytes);

                    render_pass.set_bind_group(0, &model.material.bind_group, &[]);
                    render_pass.set_bind_group(2, bones_bg, &[bone_byte_off as u32]);
                    render_pass.set_vertex_buffer(0, model.vertex_buffer.slice(..));
                    render_pass.set_vertex_buffer(
                        1,
                        self.animated_instance_buffer
                            .slice(inst_byte_off..inst_byte_off + anim_stride_u64),
                    );

                    render_pass
                        .set_index_buffer(model.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                    render_pass.draw_indexed(0..model.num_elements, 0, 0..1);

                    animated_slot += 1;
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}

fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: wgpu::ShaderModuleDescriptor,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(shader);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: vertex_layouts,
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState {
                    alpha: wgpu::BlendComponent::REPLACE,
                    color: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: Some(true),
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
    })
}
