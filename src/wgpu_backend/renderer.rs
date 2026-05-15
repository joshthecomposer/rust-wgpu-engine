use std::{collections::HashMap, mem::size_of, num::NonZeroU64, path::Path, sync::Arc};
use winit::window::Window;

use wgpu::{hal::dx12::PipelineLayout, util::DeviceExt, RenderPipeline};

use crate::{
    camera::{Camera, CameraUniform, SkyCameraUniform},
    entity_manager::EntityManager,
    enums_types::InstanceUniform,
    lights::{DirLight, DirLightUniform, Lights},
    util::constants::{FACES_CUBEMAP, MAX_BONES, SKYBOX_INDICES, SKYBOX_VERTICES},
    wgpu_backend::{
        bone_uniforms::BoneUniforms,
        cube_texture::CubeTexture,
        model::{DrawModel, Model},
        pipelines::{
            animated_model::{self, AnimatedModelResources},
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

        let shared_layouts = shared::build_layouts(&device);
        let camera = shared::build_camera_binding(&device, &shared_layouts.camera, camera_uniform);
        let dir_light =
            shared::build_dir_light_binding(&device, &shared_layouts.dir_light, dir_light_uniform);

        let skybox = skybox::build(&device, &queue, config.format, DEPTH_FORMAT);
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
            depth_texture,
            alignment,
            shared_layouts,
            camera,
            dir_light,
            skybox,
            static_model,
            animated_model,
        }
    }

    pub fn render_world(
        &mut self,
        camera: &Camera,
        em: &EntityManager,
        alpha: f32,
        lights: &Lights,
    ) {
        self.queue
            .write_buffer(&self.camera.buffer, 0, bytemuck::bytes_of(&camera.uniform));

        self.queue.write_buffer(
            &self.dir_light.buffer,
            0,
            bytemuck::bytes_of(&lights.dir_light_uniform),
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

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render encoder"),
            });

        self.skybox.render_pass(
            &mut encoder,
            &self.queue,
            &view,
            &self.depth_texture.view,
            camera,
        );

        // SCENE RENDER PASS
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

            // BIND CAMERA
            rp.set_bind_group(1, &self.camera.bind_group, &[]);

            // STATIC MODEL PASS
            rp.set_bind_group(2, &self.dir_light.bind_group, &[]);
            self.static_model.draw_all(&mut rp, &self.queue, em, alpha);

            // ANIMATED MODEL PASS
            rp.set_bind_group(3, &self.dir_light.bind_group, &[]);
            self.animated_model
                .draw_all(&mut rp, &self.queue, em, self.alignment, alpha);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
