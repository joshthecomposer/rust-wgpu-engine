use std::{collections::HashMap, sync::Arc};
use winit::window::Window;

use wgpu::{util::DeviceExt, RenderPipeline};

use crate::{
    camera::{Camera, CameraUniform},
    entity_manager::EntityManager,
    enums_types::InstanceUniform,
    wgpu_backend::{
        bind_group_layout_type::BindGroupLayoutType, model::Model, pipeline_type::PipelineType,
        vertex::Vertex,
    },
};

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
        // Pipelines
        // ==============================================
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Static Model Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../resources/shaders/model/static_model.wgsl").into(),
            ),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Model Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&texture_bind_group_layout),
                    Some(&camera_bind_group_layout),
                ],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Main render pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc(), InstanceUniform::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),

            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },

            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                // bitwise NOT 0 (all flags)
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        pipelines.insert(PipelineType::Model, render_pipeline);

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
                depth_stencil_attachment: None,

                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            let pipeline = self.pipelines.get(&PipelineType::Model).unwrap();
            let cam_bg = self.bind_groups.get(&BindGroupLayoutType::Camera).unwrap();

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(1, cam_bg, &[]);

            let ids = em.get_ids_for_type("Mountain1");

            let mut instances = vec![];
            let mut batch_model: Option<&Model> = None;

            for id in ids.iter() {
                let id = *id;
                let transform = em.transforms.get(id).unwrap();
                let model = em.models.get(id).unwrap();

                if batch_model.is_none() {
                    batch_model = Some(model);
                }

                instances.push(transform.to_instance_uniform());
            }

            if let Some(model) = batch_model {
                if !instances.is_empty() {
                    let instance_bytes = bytemuck::cast_slice(&instances);
                    self.queue
                        .write_buffer(&self.instance_buffer, 0, instance_bytes);

                    render_pass.set_bind_group(0, &model.material.bind_group, &[]);
                    render_pass.set_vertex_buffer(0, model.vertex_buffer.slice(..));
                    render_pass.set_vertex_buffer(
                        1,
                        self.instance_buffer
                            .slice(0..instance_bytes.len() as wgpu::BufferAddress),
                    );

                    render_pass
                        .set_index_buffer(model.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                    let n = instances.len() as u32;

                    render_pass.draw_indexed(0..model.num_elements, 0, 0..n);
                }
            }
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
