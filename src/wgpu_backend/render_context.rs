pub struct RenderContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub layout: &'a wgpu::BindGroupLayout,
}
