use std::{
    collections::HashMap,
    fs::{read_to_string, write},
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::enums_types::SoundType;

#[derive(Deserialize, Serialize, Debug)]
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
    pub render_gizmos: bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        let mut sounds = HashMap::new();
        sounds.insert(SoundType::MooseHuff, "event:/moose3D".to_string());
        sounds.insert(SoundType::Footstep, "event:/footstep".to_string());
        sounds.insert(SoundType::Music, "event:/music".to_string());
        sounds.insert(SoundType::StopRunning, "event:/stop_running".to_string());
        sounds.insert(SoundType::Jump, "event:/jump".to_string());
        sounds.insert(SoundType::Land, "event:/land".to_string());

        Self {
            game_title: "Spaghetti Engine".to_string(),
            cell_size: 1.0,
            win_width: 1280.0,
            win_height: 720.0,
            grid_height: 100,
            grid_width: 100,
            vsync: true,
            debug_mode: true,
            fps_counter: true,
            sounds,
            render_gizmos: false,
        }
    }
}

impl GameConfig {
    pub fn load_from_file(file_name: &str) -> GameConfig {
        println!("loading game configuration from {}", &file_name);
        let config_str = read_to_string(file_name).unwrap();

        serde_json::from_str(&config_str).expect("The gameconfig file was missing")
    }

    pub fn save_to_file(&self, file_name: &str) {
        println!("saving game configuration to {}", file_name);
        let json_string =
            serde_json::to_string_pretty(self).expect("Failed to serialize game config");
        write(file_name, json_string).expect("Failed to write game config file");
    }

    /// Load config from file if it exists, otherwise create default config and save it
    pub fn load_or_create_default(file_name: &str) -> GameConfig {
        if Path::new(file_name).exists() {
            Self::load_from_file(file_name)
        } else {
            println!(
                "Config file not found at {}, creating default config",
                file_name
            );
            let config = GameConfig::default();
            config.save_to_file(file_name);
            config
        }
    }
}
