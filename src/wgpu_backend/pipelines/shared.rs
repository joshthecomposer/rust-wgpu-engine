use wgpu::util::DeviceExt;

use crate::{camera::CameraUniform, lights::DirLightUniform};

pub struct SharedLayouts {
    pub texture: wgpu::BindGroupLayout,
    pub camera: wgpu::BindGroupLayout,
    /// Directional light + shadow map sampling (keeps animated lit pass within 4 bind groups).
    pub dir_light: wgpu::BindGroupLayout,
    pub light_space: wgpu::BindGroupLayout,
}

pub struct CameraBinding {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

pub struct DirLightBinding {
    pub buffer: wgpu::Buffer,
}

pub fn build_layouts(device: &wgpu::Device) -> SharedLayouts {
    let texture = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Texture_bind_group_layout"),
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
    });

    let camera = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("camera_bind_group_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: Some(
                    std::num::NonZeroU64::new(std::mem::size_of::<CameraUniform>() as u64)
                        .unwrap(),
                ),
            },
            count: None,
        }],
    });

    let dir_light = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("dir_light_shadow_layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Depth,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
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

    let light_space = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("light_space_layout"),
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
    });

    SharedLayouts {
        texture,
        camera,
        dir_light,
        light_space,
    }
}

pub fn build_camera_binding(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    initial: CameraUniform,
) -> CameraBinding {
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Camera buffer"),
        contents: bytemuck::cast_slice(&[initial]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("camera_bind_group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });

    CameraBinding { buffer, bind_group }
}

pub fn build_dir_light_buffer(device: &wgpu::Device, initial: DirLightUniform) -> DirLightBinding {
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Dir Light Buffer"),
        contents: bytemuck::cast_slice(&[initial]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    DirLightBinding { buffer }
}

pub fn scene_color_targets(
    scene_format: wgpu::TextureFormat,
    bright_format: wgpu::TextureFormat,
) -> [Option<wgpu::ColorTargetState>; 2] {
    [
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
    ]
}

/// Second MRT for WebGL2: stores `frag_pos.z` (same convention as the depth
/// buffer) in **.r** so the HDR composite can fog with a normal `texture_2d`
/// ( GLES `sampler2D` ) — sampling the real depth texture is not portable in
/// WGSL→GLSL for this backend.
#[cfg(target_arch = "wasm32")]
const DEPTH_PROXY_FORMAT_PREFERRED: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;
#[cfg(target_arch = "wasm32")]
const DEPTH_PROXY_FORMAT_FALLBACK: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[cfg(target_arch = "wasm32")]
pub fn select_depth_proxy_format(adapter: &wgpu::Adapter) -> wgpu::TextureFormat {
    const REQUIRED: wgpu::TextureUsages = wgpu::TextureUsages::RENDER_ATTACHMENT
        .union(wgpu::TextureUsages::TEXTURE_BINDING);

    let features = adapter.get_texture_format_features(DEPTH_PROXY_FORMAT_PREFERRED);
    if features.allowed_usages.contains(REQUIRED) {
        return DEPTH_PROXY_FORMAT_PREFERRED;
    }

    eprintln!(
        "WASM fog depth proxy: R16Float not renderable; falling back to Rgba8Unorm (fog may band on weak GPUs)"
    );
    DEPTH_PROXY_FORMAT_FALLBACK
}

#[cfg(target_arch = "wasm32")]
pub fn scene_color_targets_wasm(
    scene_format: wgpu::TextureFormat,
    bright_format: wgpu::TextureFormat,
    depth_proxy_format: wgpu::TextureFormat,
) -> [Option<wgpu::ColorTargetState>; 3] {
    let two = scene_color_targets(scene_format, bright_format);
    [
        two[0].clone(),
        two[1].clone(),
        Some(wgpu::ColorTargetState {
            format: depth_proxy_format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        }),
    ]
}
