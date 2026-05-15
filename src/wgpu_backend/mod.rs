pub mod bind_group_layout_type;
pub mod bone_uniforms;
pub mod cube_texture;
pub mod data_loader;
pub mod material;
pub mod model;
pub mod pipeline_type;
pub mod pipelines;
pub mod render_context;
pub mod renderer;
pub mod texture;
pub mod vertex;

pub struct Layouts {
    pub texture: wgpu::BindGroupLayout,
    pub camera: wgpu::BindGroupLayout,
    pub sky_cam: wgpu::BindGroupLayout,
    pub skybox: wgpu::BindGroupLayout,
    pub bones: wgpu::BindGroupLayout,
}

pub struct BindGroups {
    pub camera: wgpu::BindGroup,
    pub sky_cam: wgpu::BindGroup,
    pub skybox: wgpu::BindGroup,
    pub bones: wgpu::BindGroup,
}

pub struct Pipelines {
    pub skybox: wgpu::RenderPipeline,
    pub model: wgpu::RenderPipeline,
    pub animated_model: wgpu::RenderPipeline,
}
