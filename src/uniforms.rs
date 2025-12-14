#![allow(dead_code)]
use glam::Vec3;

#[repr(C)]
pub struct Material {
    pub diffuse: u32,
    pub specular: u32,
    pub shininess: f32,
    _padding0: f32,
}

#[repr(C)]
pub struct PointLight {
    pub position: Vec3,
    _padding0: f32,

    pub ambient: Vec3,
    _padding1: f32,

    pub diffuse: Vec3,
    _padding2: f32,

    pub specular: Vec3,
    _padding3: f32,

    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
    _padding4: f32,
}

#[repr(C)]
pub struct DirLight {
    pub direction: Vec3,
    _padding0: f32,

    pub ambient: Vec3,
    _padding1: f32,

    pub diffuse: Vec3,
    _padding2: f32,

    pub specular: Vec3,
    _padding3: f32,
}
