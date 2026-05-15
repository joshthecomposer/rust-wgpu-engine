use std::{collections::HashMap, mem::size_of, num::NonZeroU64, path::Path, sync::Arc};
use winit::window::Window;

use wgpu::{hal::dx12::PipelineLayout, util::DeviceExt, RenderPipeline};

use crate::{
    camera::{Camera, CameraUniform, SkyCameraUniform},
    entity_manager::EntityManager,
    enums_types::InstanceUniform,
    util::constants::{FACES_CUBEMAP, MAX_BONES, SKYBOX_INDICES, SKYBOX_VERTICES},
    wgpu_backend::{
        bind_group_layout_type::BindGroupLayoutType,
        bone_uniforms::BoneUniforms,
        cube_texture::CubeTexture,
        model::{DrawModel, Model},
        pipeline_type::PipelineType,
        pipelines::{animated_model, shared, skybox, static_model},
        texture,
        vertex::Vertex,
        BindGroups, Layouts, Pipelines,
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

    pub pipelines: Pipelines,
    pub layouts: Layouts,
    pub bind_groups: BindGroups,
    pub camera_buffer: wgpu::Buffer,

    pub instance_buffer: wgpu::Buffer,
    pub animated_instance_buffer: wgpu::Buffer,
    pub bones_buffer: wgpu::Buffer,
    pub depth_texture: texture::Texture,
    pub sky_cube: CubeTexture,

    pub sky_camera_buffer: wgpu::Buffer,
    pub sky_vertex_buffer: wgpu::Buffer,
    pub sky_index_buffer: wgpu::Buffer,
    pub sky_index_count: u32,

    pub alignment: usize,
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

        let shared_layouts = shared::build_layouts(&device);
        let camera = shared::build_camera_binding(&device, &shared_layouts.camera, camera_uniform);

        let sky = skybox::build(&device, &queue, config.format, DEPTH_FORMAT);
        let static_model =
            static_model::build(&device, &shared_layouts, config.format, DEPTH_FORMAT);
        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let animated_model =
            animated_model::build(&device, &shared_layouts, config.format, DEPTH_FORMAT);

        let alignment = device.limits().min_uniform_buffer_offset_alignment as usize;

        Self {
            surface,
            device,
            queue,
            config,
            layouts: Layouts {
                texture: shared_layouts.texture,
                camera: shared_layouts.camera,
                sky_cam: sky.layout,
                skybox: sky.env_layout,
                bones: animated_model.bones_layout,
            },
            bind_groups: BindGroups {
                camera: camera.bind_group,
                sky_cam: sky.camera_bind_group,
                skybox: sky.env_bind_group,
                bones: animated_model.bones_bind_group,
            },
            pipelines: Pipelines {
                skybox: sky.pipeline,
                model: static_model.pipeline,
                animated_model: animated_model.pipeline,
            },
            camera_buffer: camera.buffer,
            instance_buffer: static_model.instance_buffer,
            animated_instance_buffer: animated_model.instance_buffer,
            depth_texture,
            sky_cube: sky.cube,
            bones_buffer: animated_model.bones_buffer,
            alignment,
            sky_camera_buffer: sky.camera_buffer,
            sky_vertex_buffer: sky.vertex_buffer,
            sky_index_buffer: sky.index_buffer,
            sky_index_count: sky.index_count,
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
        // SKYBOX RENDER PASS
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
                            g: 0.0,
                            b: 1.0,
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

            let sky_uniform = SkyCameraUniform::from_camera(camera);
            self.queue
                .write_buffer(&self.sky_camera_buffer, 0, bytemuck::bytes_of(&sky_uniform));
            render_pass.set_pipeline(&self.pipelines.skybox);
            render_pass.set_bind_group(0, &self.bind_groups.sky_cam, &[]);
            render_pass.set_bind_group(1, &self.bind_groups.skybox, &[]);
            render_pass.set_vertex_buffer(0, self.sky_vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(self.sky_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.sky_index_count, 0, 0..1);
        }

        // ==============================================
        // MAIN RENDER PASS | MODEL RENDER PASS
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

            // CAMERA

            render_pass.set_bind_group(1, &self.bind_groups.camera, &[]);

            // STATIC PASS

            let instance_stride = std::mem::size_of::<InstanceUniform>() as wgpu::BufferAddress;

            let mut instance_byte_offset: wgpu::BufferAddress = 0;

            render_pass.set_pipeline(&self.pipelines.model);

            let ids_by_type = em.get_modeled_static_ids_by_type();

            for ids in ids_by_type.values() {
                if ids.is_empty() {
                    continue;
                }

                let mut instances = Vec::new();

                let mut batch_model: Option<&Model> = None;

                for &id in ids.iter() {
                    let (Some(transform), Some(model)) = (em.transforms.get(id), em.models.get(id))
                    else {
                        continue;
                    };
                    if batch_model.is_none() {
                        batch_model = Some(model);
                    }
                    instances.push(transform.to_instance_uniform());
                }

                let Some(model) = batch_model else { continue };

                if instances.is_empty() {
                    continue;
                }

                let instance_bytes = bytemuck::cast_slice(&instances);

                let batch_bytes = instance_bytes.len() as wgpu::BufferAddress;

                debug_assert!(
                    instance_byte_offset + batch_bytes <= self.instance_buffer.size(),
                    "instance_buffer too small"
                );

                self.queue.write_buffer(
                    &self.instance_buffer,
                    instance_byte_offset,
                    instance_bytes,
                );

                render_pass.set_vertex_buffer(
                    1,
                    self.instance_buffer
                        .slice(instance_byte_offset..instance_byte_offset + batch_bytes),
                );

                render_pass.draw_model_instanced(model, 0..instances.len() as u32);

                instance_byte_offset += instances.len() as wgpu::BufferAddress * instance_stride;
            }

            // ANIMATED PASS
            render_pass.set_pipeline(&self.pipelines.animated_model);
            instance_byte_offset = 0;

            let mut bones_byte_offset: wgpu::BufferAddress = 0;

            let bone_stride = (size_of::<BoneUniforms>() as wgpu::BufferAddress)
                .next_multiple_of(self.alignment as wgpu::BufferAddress);

            let ids_by_type = em.get_animated_ids_by_type();

            for ids in ids_by_type.values() {
                for id in ids {
                    let model = em.models.get(*id).unwrap();
                    let animator = em.animators.get(*id).unwrap();
                    let anim = animator.get_current_animation().unwrap();

                    let transform = em.transforms.get(*id).unwrap();

                    let instance = transform.to_instance_uniform();

                    self.queue.write_buffer(
                        &self.animated_instance_buffer,
                        instance_byte_offset,
                        bytemuck::cast_slice(&[instance]),
                    );

                    self.queue.write_buffer(
                        &self.bones_buffer,
                        bones_byte_offset,
                        bytemuck::cast_slice(&anim.current_pose),
                    );

                    render_pass.set_vertex_buffer(
                        1,
                        self.animated_instance_buffer
                            .slice(instance_byte_offset..instance_byte_offset + instance_stride),
                    );

                    let bones_dynamic_offset: wgpu::DynamicOffset = bones_byte_offset
                        .try_into()
                        .expect("bones slab offset fits u32");

                    render_pass.draw_model_animated(
                        model,
                        &self.bind_groups.bones,
                        bones_dynamic_offset,
                    );

                    instance_byte_offset += instance_stride;
                    bones_byte_offset += bone_stride;
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
