use std::{collections::HashMap, mem::size_of, num::NonZeroU64, path::Path, sync::Arc};
use winit::window::Window;

use wgpu::util::DeviceExt;

use crate::{
    camera::{Camera, CameraUniform, SkyCameraUniform},
    entity_manager::EntityManager,
    enums_types::InstanceUniform,
    lights::{DirLight, DirLightUniform, Lights},
    particles::ParticleSystem,
    util::constants::{
        FACES_CUBEMAP, MAX_BONES, SHADOW_HEIGHT, SHADOW_WIDTH, SKYBOX_INDICES, SKYBOX_VERTICES,
    },
    wgpu_backend::{
        bone_uniforms::BoneUniforms,
        cube_texture::CubeTexture,
        gizmo::GizmoRenderer,
        model::{DrawModel, Model},
        pipelines::{
            animated_model::{self, AnimatedBatch, AnimatedModelResources},
            bloom::{self, BloomResources},
            gizmo as gizmo_pipeline,
            hdr::{self, HdrCompositeParams, HdrResources},
            particles::{self, ParticleResources},
            shadows::{self, ShadowResources},
            shared::{self, CameraBinding, DirLightBinding, SharedLayouts},
            skybox::{self, SkyboxResources},
            static_model::{self, StaticBatch, StaticModelResources},
        },
        texture,
        vertex::Vertex,
        world_draws::WorldDraws,
        BindGroups, Layouts, Pipelines,
    },
};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

const SCENE_HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

const BRIGHT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

const HDR_ENABLED: u32 = 1;

pub struct Renderer {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    /// sRGB view format the swapchain is rendered through. The surface itself is
    /// configured with the non-sRGB base format (required for the WebGPU canvas),
    /// but we present through an sRGB view so linear color is gamma-encoded on
    /// write — identically on native and web.
    pub surface_view_format: wgpu::TextureFormat,
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
    pub gizmo: GizmoRenderer,
    pub render_gizmos: bool,
    pub shadows: ShadowResources,
    pub world_draws: WorldDraws,
}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        camera_uniform: CameraUniform,
        dir_light_uniform: DirLightUniform,
    ) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let (width, height) = {
            let inner = window.inner_size();
            (inner.width.max(1), inner.height.max(1))
        };
        #[cfg(target_arch = "wasm32")]
        let (width, height) = crate::platform::web_canvas_physical_size(&window);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::BROWSER_WEBGPU,
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
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let chosen_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .or_else(|| surface_caps.formats.first().copied())
            .unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb);

        // Configure the canvas/swapchain with the non-sRGB base (WebGPU only
        // permits non-sRGB canvas formats), but render through an sRGB view so the
        // GPU gamma-encodes linear output. Native already did this implicitly via
        // an sRGB surface format; doing it explicitly keeps both backends in sync.
        let base_format = chosen_format.remove_srgb_suffix();
        let surface_view_format = base_format.add_srgb_suffix();

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
            format: base_format,
            width,
            height,
            present_mode,
            alpha_mode,
            view_formats: vec![surface_view_format],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let mut initial_hdr_params = HdrCompositeParams::new();
        initial_hdr_params.hdr_enabled = HDR_ENABLED;
        let mut hdr = hdr::build(
            &device,
            &queue,
            config.width,
            config.height,
            surface_view_format,
            SCENE_HDR_FORMAT,
            BRIGHT_FORMAT,
            initial_hdr_params,
            &depth_texture.view,
        );

        let bloom = bloom::build(
            &device,
            config.width,
            config.height,
            BRIGHT_FORMAT,
            hdr.bright_view(),
        );
        hdr.set_bloom_view(&device, bloom.output_view(), &depth_texture.view);

        let scene_hdr_format = hdr.scene_format;

        let shared_layouts = shared::build_layouts(&device);
        let camera = shared::build_camera_binding(&device, &shared_layouts.camera, camera_uniform);
        let dir_light = shared::build_dir_light_buffer(&device, dir_light_uniform);

        let skybox = skybox::build(
            &device,
            &queue,
            scene_hdr_format,
            BRIGHT_FORMAT,
            DEPTH_FORMAT,
        );
        let static_model = static_model::build(
            &device,
            &shared_layouts,
            scene_hdr_format,
            BRIGHT_FORMAT,
            DEPTH_FORMAT,
        );

        let animated_model = animated_model::build(
            &device,
            &shared_layouts,
            scene_hdr_format,
            BRIGHT_FORMAT,
            DEPTH_FORMAT,
        );

        let shadows = ShadowResources::build(
            &device,
            &shared_layouts,
            &dir_light.buffer,
            &animated_model.bones_layout,
        );

        let particles = particles::build(
            &device,
            &queue,
            &shared_layouts,
            hdr.scene_format,
            hdr.bright_format,
            DEPTH_FORMAT,
        );

        let gizmo = GizmoRenderer::new(
            gizmo_pipeline::build(
                &device,
                &shared_layouts,
                hdr.scene_format,
                hdr.bright_format,
                DEPTH_FORMAT,
            ),
            &device,
        );

        let alignment = device.limits().min_uniform_buffer_offset_alignment as usize;

        Self {
            surface,
            device,
            queue,
            config,
            surface_view_format,
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
            gizmo,
            render_gizmos: false,
            shadows,
            world_draws: WorldDraws::new(),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        if self.config.width == width && self.config.height == height {
            return;
        }

        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);

        self.depth_texture =
            texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");

        let mut hdr_params = HdrCompositeParams::new();
        hdr_params.hdr_enabled = HDR_ENABLED;
        self.hdr = hdr::build(
            &self.device,
            &self.queue,
            self.config.width,
            self.config.height,
            self.surface_view_format,
            SCENE_HDR_FORMAT,
            BRIGHT_FORMAT,
            hdr_params,
            &self.depth_texture.view,
        );

        self.bloom = bloom::build(
            &self.device,
            self.config.width,
            self.config.height,
            BRIGHT_FORMAT,
            self.hdr.bright_view(),
        );

        self.hdr.set_bloom_view(
            &self.device,
            self.bloom.output_view(),
            &self.depth_texture.view,
        );
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
        let light_space = ShadowResources::compute_light_space(lights, camera, alpha);
        self.shadows.write_light_space(&self.queue, light_space);
        self.shadows.write_sample_uniforms(&self.queue, lights);

        let mut camera_uniform = camera.uniform;
        camera_uniform.light_view_proj = light_space.to_cols_array_2d();
        self.queue
            .write_buffer(&self.camera.buffer, 0, bytemuck::bytes_of(&camera_uniform));

        self.queue.write_buffer(
            &self.dir_light.buffer,
            0,
            bytemuck::bytes_of(&lights.dir_light_uniform),
        );

        let mut static_batches: Vec<StaticBatch<'_>> = Vec::new();
        let mut animated_batches: Vec<AnimatedBatch<'_>> = Vec::new();
        self.static_model.prepare(
            &self.queue,
            em,
            alpha,
            &mut self.world_draws.static_scratch,
            &mut static_batches,
        );
        self.animated_model.prepare(
            &self.queue,
            em,
            self.alignment,
            alpha,
            &mut animated_batches,
        );

        // Texture uploads must happen before begin_render_pass.
        self.particles.upload_pending_textures(
            &self.device,
            &self.queue,
            &self.shared_layouts.texture,
            particles,
        );

        // GPU buffer uploads for gizmos must also happen before begin_render_pass.
        // The toggle is checked here so prepare work (and the per-entity instance
        // upload) is skipped entirely when gizmo rendering is off.
        if self.render_gizmos {
            self.gizmo.prepare(&self.device, &self.queue, em, alpha);
        }

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

        let surface_view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.surface_view_format),
            ..Default::default()
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render encoder"),
            });

        {
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadows.depth_view,
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
            shadow_pass.set_viewport(
                0.0,
                0.0,
                SHADOW_WIDTH as f32,
                SHADOW_HEIGHT as f32,
                0.0,
                1.0,
            );
            shadow_pass.set_bind_group(0, &self.shadows.light_bind_group, &[]);
            self.static_model.draw_prepared(
                &mut shadow_pass,
                &self.shadows.static_pipeline,
                &static_batches,
                false,
            );
            self.animated_model.draw_prepared(
                &mut shadow_pass,
                &self.shadows.animated_pipeline,
                &animated_batches,
                false,
            );
        }

        let hdr_view = self.hdr.scene_view();
        let bright_view = self.hdr.bright_view();

        // write to hdr view
        self.skybox.render_pass(
            &mut encoder,
            &self.queue,
            hdr_view,
            bright_view,
            &self.depth_texture.view,
            camera,
        );

        // SCENE RENDER PASS — opaque + translucent in one 2-target pass.
        let scene_color_attachments: [Option<wgpu::RenderPassColorAttachment>; 2] = [
            Some(wgpu::RenderPassColorAttachment {
                view: hdr_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: bright_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
        ];

        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &scene_color_attachments,
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
            rp.set_bind_group(2, &self.shadows.lighting_bind_group, &[]);
            self.static_model.draw_prepared(
                &mut rp,
                &self.static_model.pipeline,
                &static_batches,
                true,
            );

            // ANIMATED MODEL PASS
            rp.set_bind_group(3, &self.shadows.lighting_bind_group, &[]);
            self.animated_model.draw_prepared(
                &mut rp,
                &self.animated_model.pipeline,
                &animated_batches,
                true,
            );

            // PARTICLES PASS
            // alpha-blended on scene, additive on bright. Depth-test on, depth-write off.
            self.particles
                .draw_all(&mut rp, &self.queue, particles, camera);

            // GIZMO PASS
            // LineList wireframes for colliders.
            if self.render_gizmos {
                self.gizmo.render(&mut rp, &self.camera.bind_group);
            }
        }

        let _bloom_view = self.bloom.render(&mut encoder, &self.queue);

        self.hdr.write_params(
            &self.queue,
            HdrCompositeParams {
                exposure: 1.0,
                bloom_strength: 0.1,
                hdr_enabled: HDR_ENABLED,
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
