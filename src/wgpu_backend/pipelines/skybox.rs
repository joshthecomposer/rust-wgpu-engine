use std::num::NonZeroU64;

use wgpu::util::DeviceExt;

use crate::{
    assets,
    camera::{Camera, SkyCameraUniform},
    util::constants::{FACES_CUBEMAP, SKYBOX_INDICES, SKYBOX_VERTICES},
    wgpu_backend::{
        cube_texture::CubeTexture,
        pipelines::{create_render_pipeline, shared},
    },
};

pub struct SkyboxResources {
    pub layout: wgpu::BindGroupLayout,
    pub env_layout: wgpu::BindGroupLayout,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub env_bind_group: wgpu::BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub cube: CubeTexture,
    pub pipeline: wgpu::RenderPipeline,
}

impl SkyboxResources {
    pub fn render_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        color_view: &wgpu::TextureView,
        bright_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        camera: &Camera,
    ) {
        let sky_uniform = SkyCameraUniform::from_camera(camera);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&sky_uniform));

        let hdr_att = Some(wgpu::RenderPassColorAttachment {
            view: color_view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        });
        let bright_att = Some(wgpu::RenderPassColorAttachment {
            view: bright_view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        });

        let color_attachments: [Option<wgpu::RenderPassColorAttachment>; 2] = [hdr_att, bright_att];

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("skybox pass"),
            color_attachments: &color_attachments,
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
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

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.env_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

pub fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene_format: wgpu::TextureFormat,
    bright_format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
) -> SkyboxResources {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sky_cam_bind_group_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: Some(
                    NonZeroU64::new(size_of::<SkyCameraUniform>() as u64).unwrap(),
                ),
            },
            count: None,
        }],
    });

    let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sky_camera_buffer"),
        size: size_of::<SkyCameraUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sky_cam_bind_group"),
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
    });

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("skybox_vertices"),
        contents: bytemuck::cast_slice(&SKYBOX_VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("skybox_indices"),
        contents: bytemuck::cast_slice(&SKYBOX_INDICES),
        usage: wgpu::BufferUsages::INDEX,
    });

    let cube = load_skybox_ldr_separated_faces(device, queue);

    let env_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("skybox bind group layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let env_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("skybox bind group"),
        layout: &env_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(cube.view()),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(cube.sampler()),
            },
        ],
    });

    let position_layout = wgpu::VertexBufferLayout {
        array_stride: size_of::<[f32; 3]>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x3,
        }],
    };

    let sky_wgsl: &str = include_str!("../../../resources/shaders/skybox.wgsl");

    let shader = wgpu::ShaderModuleDescriptor {
        label: Some("Skybox shader"),
        source: wgpu::ShaderSource::Wgsl(sky_wgsl.into()),
    };

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Skybox Pipeline Layout"),
        bind_group_layouts: &[Some(&layout), Some(&env_layout)],
        immediate_size: 0,
    });

    let scene_targets = shared::scene_color_targets(scene_format, bright_format);

    let pipeline = create_render_pipeline(
        &device,
        &pipeline_layout,
        &scene_targets,
        Some(depth_format),
        &[position_layout],
        shader,
        Some("Skybox Pipeline"),
        Some(wgpu::CompareFunction::LessEqual),
        None,
    );

    SkyboxResources {
        layout,
        env_layout,
        camera_buffer,
        camera_bind_group,
        env_bind_group,
        vertex_buffer,
        index_buffer,
        index_count: SKYBOX_INDICES.len() as u32,
        cube,
        pipeline,
    }
}

fn load_skybox_ldr_separated_faces(device: &wgpu::Device, queue: &wgpu::Queue) -> CubeTexture {
    // Route through `assets::read_bytes` so the same code works on native
    // (filesystem) and wasm (browser-preloaded asset map populated by
    // index.html before `init()`).
    let first_bytes = assets::read_bytes(FACES_CUBEMAP[0]).unwrap();

    let first = image::load_from_memory(&first_bytes).unwrap().to_rgba8();

    let (cube_w, cube_h) = first.dimensions();

    let sky_fmt = wgpu::TextureFormat::Rgba8UnormSrgb;
    let cube = CubeTexture::create_2d(
        device,
        cube_w,
        cube_h,
        sky_fmt,
        1,
        wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        wgpu::FilterMode::Linear,
        Some("Skybox cube"),
    );

    for (layer, path) in FACES_CUBEMAP.iter().enumerate() {
        let bytes = assets::read_bytes(path).unwrap();

        let rgba = image::load_from_memory(&bytes).unwrap().to_rgba8();

        let dst_origin = wgpu::Origin3d {
            x: 0,
            y: 0,
            z: layer as u32,
        };

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: cube.texture(),
                mip_level: 0,
                origin: dst_origin,
                aspect: wgpu::TextureAspect::All,
            },
            rgba.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * cube_w),
                rows_per_image: Some(cube_h),
            },
            wgpu::Extent3d {
                width: cube_w,
                height: cube_h,
                depth_or_array_layers: 1,
            },
        );
    }
    cube
}
