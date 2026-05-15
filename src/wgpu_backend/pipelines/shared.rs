use wgpu::util::DeviceExt;

use crate::camera::CameraUniform;

pub struct SharedLayouts {
    pub texture: wgpu::BindGroupLayout,
    pub camera: wgpu::BindGroupLayout,
}

pub struct CameraBinding {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
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
                min_binding_size: None,
            },
            count: None,
        }],
    });

    SharedLayouts { texture, camera }
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
