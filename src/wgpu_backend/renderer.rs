use std::{collections::HashMap, mem::size_of, num::NonZeroU64, path::Path, sync::Arc};
use winit::window::Window;

use wgpu::{hal::dx12::PipelineLayout, util::DeviceExt, RenderPipeline};

use crate::{
    camera::{Camera, CameraUniform, SkyCameraUniform},
    entity_manager::EntityManager,
    enums_types::InstanceUniform,
    lights::{DirLight, DirLightUniform, Lights},
    particles::ParticleSystem,
    util::constants::{FACES_CUBEMAP, MAX_BONES, SKYBOX_INDICES, SKYBOX_VERTICES},
    wgpu_backend::{
        bone_uniforms::BoneUniforms,
        cube_texture::CubeTexture,
        model::{DrawModel, Model},
        pipelines::{
            animated_model::{self, AnimatedModelResources},
            bloom::{self, BloomResources},
            hdr::{self, HdrCompositeParams, HdrResources},
            particles::{self, ParticleResources},
            shared::{self, CameraBinding, DirLightBinding, SharedLayouts},
            skybox::{self, SkyboxResources},
            static_model::{self, StaticModelResources},
        },
        texture,
        vertex::Vertex,
        BindGroups, Layouts, Pipelines,
    },
};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct Renderer {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub depth_texture: texture::Texture,
    pub alignment: usize,
    pub shared_layouts: SharedLayouts,
    pub camera: CameraBinding,
    pub dir_light: DirLightBinding,
    pub skybox: SkyboxResources,
    pub static_model: StaticModelResources,
    pub animated_model: AnimatedModelResources,
    pub particles: ParticleResources,
    pub bloom: BloomResources,
    pub hdr: HdrResources,
}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        camera_uniform: CameraUniform,
        dir_light_uniform: DirLightUniform,
    ) -> Self {
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

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let mut hdr = hdr::build(
            &device,
            &queue,
            config.width,
            config.height,
            config.format,
            HdrCompositeParams::new(),
            &depth_texture.view,
        );

        let bloom = bloom::build(&device, config.width, config.height, hdr.bright_view());
        hdr.set_bloom_view(&device, bloom.output_view(), &depth_texture.view);

        let scene_hdr_format = hdr.scene_format;

        let shared_layouts = shared::build_layouts(&device);
        let camera = shared::build_camera_binding(&device, &shared_layouts.camera, camera_uniform);
        let dir_light =
            shared::build_dir_light_binding(&device, &shared_layouts.dir_light, dir_light_uniform);

        let skybox = skybox::build(&device, &queue, scene_hdr_format, DEPTH_FORMAT);
        let static_model =
            static_model::build(&device, &shared_layouts, scene_hdr_format, DEPTH_FORMAT);

        let animated_model =
            animated_model::build(&device, &shared_layouts, scene_hdr_format, DEPTH_FORMAT);

        let particles = particles::build(
            &device,
            &queue,
            &shared_layouts,
            hdr.scene_format,
            hdr.bright_format,
            DEPTH_FORMAT,
        );

        let alignment = device.limits().min_uniform_buffer_offset_alignment as usize;

        Self {
            surface,
            device,
            queue,
            config,
            depth_texture,
            alignment,
            shared_layouts,
            camera,
            dir_light,
            skybox,
            static_model,
            animated_model,
            particles,
            bloom,
            hdr,
        }
    }

    pub fn render_world(
        &mut self,
        camera: &Camera,
        em: &EntityManager,
        alpha: f32,
        lights: &Lights,
        particles: &mut ParticleSystem,
    ) {
        self.render_world_with_overlay(
            camera,
            em,
            alpha,
            lights,
            particles,
            None::<fn(&wgpu::Device, &wgpu::Queue, &mut wgpu::CommandEncoder, &wgpu::TextureView)>,
        );
    }

    pub fn render_world_with_overlay<F>(
        &mut self,
        camera: &Camera,
        em: &EntityManager,
        alpha: f32,
        lights: &Lights,
        particles: &mut ParticleSystem,
        render_overlay: Option<F>,
    ) where
        F: FnOnce(&wgpu::Device, &wgpu::Queue, &mut wgpu::CommandEncoder, &wgpu::TextureView),
    {
        self.queue
            .write_buffer(&self.camera.buffer, 0, bytemuck::bytes_of(&camera.uniform));

        self.queue.write_buffer(
            &self.dir_light.buffer,
            0,
            bytemuck::bytes_of(&lights.dir_light_uniform),
        );

        // Texture uploads must happen before begin_render_pass.
        self.particles.upload_pending_textures(
            &self.device,
            &self.queue,
            &self.shared_layouts.texture,
            particles,
        );

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

        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render encoder"),
            });

        let hdr_view = self.hdr.scene_view();
        let bright_view = self.hdr.bright_view();

        // write to hdr view
        self.skybox.render_pass(
            &mut encoder,
            &self.queue,
            &hdr_view,
            &bright_view,
            &self.depth_texture.view,
            camera,
        );

        // SCENE RENDER PASS
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &hdr_view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &bright_view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],

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

            // BIND CAMERA
            rp.set_bind_group(1, &self.camera.bind_group, &[]);

            // STATIC MODEL PASS
            // write to hdr view
            rp.set_bind_group(2, &self.dir_light.bind_group, &[]);
            self.static_model.draw_all(&mut rp, &self.queue, em, alpha);

            // ANIMATED MODEL PASS
            // write to hdr view
            rp.set_bind_group(3, &self.dir_light.bind_group, &[]);
            self.animated_model
                .draw_all(&mut rp, &self.queue, em, self.alignment, alpha);

            // PARTICLES PASS
            // alpha-blended on scene, additive on bright. Depth-test on, depth-write off.
            self.particles
                .draw_all(&mut rp, &self.queue, particles, camera);
        }

        let _bloom_view = self.bloom.render(&mut encoder, &self.queue);

        self.hdr.write_params(
            &self.queue,
            HdrCompositeParams {
                exposure: 1.0,
                bloom_strength: 0.1,
                hdr_enabled: 1,
                _pad0: 0,
                inv_proj: camera.uniform.inv_proj,
            },
        );

        // write hdr view to main surface_view
        self.hdr.composite_pass(&mut encoder, &surface_view);

        if let Some(render_overlay) = render_overlay {
            render_overlay(&self.device, &self.queue, &mut encoder, &surface_view);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
