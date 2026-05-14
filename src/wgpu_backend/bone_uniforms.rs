#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BoneUniforms {
    pub matrices: [glam::Mat4; 200],
}
