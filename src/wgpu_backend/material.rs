use crate::wgpu_backend::texture::Texture;

#[derive(Clone)]
pub struct Material {
    pub diffuse_texture: Texture,
    pub bind_group: wgpu::BindGroup,
}
