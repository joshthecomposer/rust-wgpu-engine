use core::fmt;
use std::{collections::HashMap, fmt::Display, fs::{read_to_string, write}, hash::Hash};
use glam::{Quat, Vec2, Vec3, Vec4};
use image::{GenericImageView, Rgba};
use russimp::Color4D;
use serde_json::ser::Formatter;
use toml::value::{Table, Value, Array};

use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

use crate::{enums_types::{EmitterName, EntityType, Faction}, gl_call};


#[derive(Deserialize, Debug, Serialize)]
pub struct EmitterData {
    pub one_shot_data: HashMap<String, EmitterBlackboard>,
}

impl EmitterData {
    pub fn load_from_file(file_name: &str) -> Self {
        println!("loading EmitterData from {}", &file_name);
        let config_str = read_to_string(file_name).unwrap();

        toml::from_str(&config_str).expect("The EmitterData file was missing or malformed")
    }
}

#[derive(Deserialize, Debug, Serialize)]
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
    #[serde(default, deserialize_with = "load_texture")]
    pub texture: Option<u32>,
}

fn load_texture<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let path = match Option::<String>::deserialize(deserializer)? {
        Some(path) => path,
        None       => return Ok(None)
    };

    let mut tex = 0;

    println!("FOUND TEXTURE {}", &path);

    unsafe {
        gl_call!(gl::GenTextures(1, &mut tex));
        gl_call!(gl::BindTexture(gl::TEXTURE_2D, tex));
        gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32));
        gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32));
        gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32));
        gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32));

        let img = match image::open(path) {
            Ok(img) => img,
            _ => panic!("error opening smoke texture"),
        };

        let (img_width, img_height) = img.dimensions();
        let rgba = img.to_rgba8();
        let raw = rgba.as_raw();

        gl_call!(gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA8 as i32,
            img_width as i32,
            img_height as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            raw.as_ptr().cast(),
        ));
    }
    Ok(Some(tex))
}
