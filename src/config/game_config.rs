use std::{fs::read_to_string, collections::HashMap};

use serde::Deserialize;

use crate::enums_types::SoundType;

#[derive(Deserialize, Debug)]
pub struct GameConfig {
    pub game_title: String,
    pub cell_size: f32,
    pub win_width: f32,
    pub win_height: f32,
    pub grid_height: usize,
    pub grid_width: usize,
    pub vsync: bool,
    pub debug_mode: bool,
    pub fps_counter: bool,
    pub sounds: HashMap<SoundType, String>,
}

impl GameConfig {
    pub fn load_from_file(file_name: &str) -> GameConfig {
        println!("loading game configuration from {}", &file_name);
        let config_str = read_to_string(file_name).unwrap();

        serde_json::from_str(&config_str).expect("The gameconfig file was missing")
    }
}
