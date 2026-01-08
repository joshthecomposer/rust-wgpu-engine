use crate::config::Config;
use glam::{Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Debug, Serialize)]
pub struct EmitterData {
    pub one_shot_data: HashMap<String, EmitterBlackboard>,
}

impl Default for EmitterData {
    fn default() -> Self {
        Self {
            one_shot_data: HashMap::new(),
        }
    }
}

impl Config for EmitterData {}

#[derive(Deserialize, Debug, Serialize, Default, Clone)]
#[serde(default)]
pub struct EmitterBlackboard {
    pub name: String,
    pub angle_rand: Vec2,
    pub radius_rand: Vec2,
    pub gravity: f32,
    pub velocity: Vec<Vec2>,
    pub particle_lifetime: Vec2,
    pub particle_scale: Vec2,
    pub particle_count: usize,
    pub colors: Vec<Vec4>,
    #[serde(default)]
    pub texture_path: Option<String>,
    #[serde(default)]
    pub texture_idx: Option<u32>,
    pub texture_has_alpha: bool,
    pub radial_speed: Vec2,
    pub up_speed: Vec2,
    pub jitter: Vec2,

    pub base_alpha: Vec2,
    pub alpha_multiplier: f32,
    pub alpha_power: f32,

    pub base_scale: Vec2,
    pub scale_multiplier: f32,
    pub scale_power: f32,

    pub direction: Vec3,
    pub pps: Option<usize>,
    pub has_bloom: bool,
}

// fn load_texture<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
// where
//     D: serde::Deserializer<'de>,
// {
//     let path = match Option::<String>::deserialize(deserializer)? {
//         Some(path) => path,
//         None => return Ok(None),
//     };

//     let mut tex = 0;

//     println!("FOUND TEXTURE {}", &path);

//     unsafe {
//         gl_call!(gl::GenTextures(1, &mut tex));
//         gl_call!(gl::BindTexture(gl::TEXTURE_2D, tex));
//         gl_call!(gl::TexParameteri(
//             gl::TEXTURE_2D,
//             gl::TEXTURE_MIN_FILTER,
//             gl::LINEAR as i32
//         ));
//         gl_call!(gl::TexParameteri(
//             gl::TEXTURE_2D,
//             gl::TEXTURE_MAG_FILTER,
//             gl::LINEAR as i32
//         ));
//         gl_call!(gl::TexParameteri(
//             gl::TEXTURE_2D,
//             gl::TEXTURE_WRAP_S,
//             gl::CLAMP_TO_EDGE as i32
//         ));
//         gl_call!(gl::TexParameteri(
//             gl::TEXTURE_2D,
//             gl::TEXTURE_WRAP_T,
//             gl::CLAMP_TO_EDGE as i32
//         ));

//         let img = match image::open(path) {
//             Ok(img) => img,
//             _ => panic!("error opening smoke texture"),
//         };

//         let (img_width, img_height) = img.dimensions();
//         let rgba = img.to_rgba8();
//         let raw = rgba.as_raw();

//         gl_call!(gl::TexImage2D(
//             gl::TEXTURE_2D,
//             0,
//             gl::RGBA8 as i32,
//             img_width as i32,
//             img_height as i32,
//             0,
//             gl::RGBA,
//             gl::UNSIGNED_BYTE,
//             raw.as_ptr().cast(),
//         ));
//     }
//     Ok(Some(tex))
// }

#[derive(Deserialize, Debug, Serialize)]
pub struct UiEmitterBlackboard {
    pub id: Option<usize>,
    pub name: String,
    pub angle_rand: [f32; 2],
    pub radius_rand: [f32; 2],
    pub gravity: f32,
    pub velocity_x: [f32; 2],
    pub velocity_y: [f32; 2],
    pub velocity_z: [f32; 2],
    pub particle_lifetime: [f32; 2],
    // TODO: THis field isn't needed any longer
    pub particle_scale: [f32; 2],
    pub particle_count: i32,
    pub colors: Vec<[f32; 4]>,
    pub texture_path: String,
    pub texture_has_alpha: bool,
    pub radial_speed: [f32; 2],
    pub up_speed: [f32; 2],
    pub jitter: [f32; 2],

    pub base_alpha: [f32; 2],  // start alpha
    pub alpha_multiplier: f32, // where we end up
    pub alpha_power: f32,      // Curve shape 1.0 is linear

    pub base_scale: [f32; 2],
    pub scale_multiplier: f32, // Where we end up in the lifetime
    pub scale_power: f32,      // curve shape 1.0 is linear

    pub direction: [f32; 3],

    // !!! Having a value > 0 makes this a continuous emitter !!!
    pub pps: i32,
    pub has_bloom: bool,
    pub origin: [f32; 3],
}

impl Default for UiEmitterBlackboard {
    fn default() -> Self {
        Self {
            id: None,
            name: String::new(),
            angle_rand: [0.0, std::f32::consts::TAU],
            radius_rand: [0.0, 0.0],
            gravity: 0.0,
            velocity_x: [0.0, 0.0],
            velocity_y: [1.0, 2.0],
            velocity_z: [0.0, 0.0],
            particle_lifetime: [0.3, 1.0],
            particle_scale: [0.0, 0.0],
            particle_count: 10,
            colors: vec![],
            texture_path: String::new(),
            texture_has_alpha: false,
            radial_speed: [0.0, 0.0],
            up_speed: [1.0, 2.0],
            jitter: [0.01, 0.2],

            base_alpha: [1.0, 1.0],
            alpha_multiplier: 0.0,
            alpha_power: 1.0,

            base_scale: [0.08, 0.1],
            scale_multiplier: 1.0,
            scale_power: 1.0,

            direction: [0.0, 1.0, 0.0],

            pps: 0,
            has_bloom: false,
            origin: [0.0, 0.0, 0.0],
        }
    }
}
