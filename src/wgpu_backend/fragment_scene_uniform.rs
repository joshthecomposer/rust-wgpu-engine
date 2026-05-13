use glam::{vec3, Vec3};

use crate::{
    camera::Camera,
    lights::{DirLight, Lights},
    util::constants::WHITE,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FragmentSceneUniform {
    pub dir_direction: Vec3,
    pub _pad0: f32,
    pub ambient: Vec3,
    pub _pad1: f32,
    pub diffuse: Vec3,
    pub _pad2: f32,
    pub specular_light: Vec3,
    pub _pad3: f32,
    pub view_position: Vec3,
    pub elapsed: f32,
    /// x=flash_white, y=selection_fresnel, z=do_reg_fresnel (1.0 = on)
    pub effect_flags_xyz: Vec3,
    pub skybox_ambient_blend: f32,
}

impl FragmentSceneUniform {
    /// Until the first [`from_lights_and_camera`] upload, buffers can start as this.
    pub fn bootstrap_placeholder() -> Self {
        Self::from_dir_light_fields(
            vec3(0.722, 1.0, 0.33).normalize_or_zero(),
            Vec3::splat(0.15),
            WHITE,
            WHITE,
            Vec3::ZERO,
            0.0,
        )
    }

    pub fn from_lights_and_camera(lights: &Lights, camera: &Camera, elapsed: f32) -> Self {
        let d = lights.dir_light.direction.normalize();
        Self::from_dir_light_fields(
            d,
            lights.dir_light.ambient,
            lights.dir_light.diffuse,
            lights.dir_light.specular,
            camera.position,
            elapsed,
        )
    }

    fn from_dir_light_fields(
        dir_direction: Vec3,
        ambient: Vec3,
        diffuse: Vec3,
        specular_light: Vec3,
        view_position: Vec3,
        elapsed: f32,
    ) -> Self {
        Self {
            dir_direction,
            _pad0: 0.0,
            ambient,
            _pad1: 0.0,
            diffuse,
            _pad2: 0.0,
            specular_light,
            _pad3: 0.0,
            view_position,
            elapsed,
            effect_flags_xyz: Vec3::ZERO,
            skybox_ambient_blend: 0.0,
        }
    }

    #[allow(dead_code)]
    pub fn neutral(camera: &Camera, elapsed: f32) -> Self {
        let d = DirLight::default_white();
        Self::from_dir_light_fields(
            d.direction.normalize(),
            d.ambient,
            d.diffuse,
            d.specular,
            camera.position,
            elapsed,
        )
    }
}
