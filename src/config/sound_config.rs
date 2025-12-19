use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::enums_types::SoundType;

#[derive(Deserialize, Serialize, Debug)]
pub struct SoundConfig {
    pub sounds: HashMap<SoundType, String>,
    pub master_volume: f32,
    pub bgm: f32,
    pub sfx: f32,
    pub voice: f32,
    pub ui: f32,
    pub ambience: f32,
}

impl Default for SoundConfig {
    fn default() -> Self {
        let mut sounds = HashMap::new();
        sounds.insert(SoundType::MooseHuff, "event:/moose3D".to_string());
        sounds.insert(SoundType::Footstep, "event:/footstep".to_string());
        sounds.insert(SoundType::Music, "event:/music".to_string());
        sounds.insert(SoundType::StopRunning, "event:/stop_running".to_string());
        sounds.insert(SoundType::Jump, "event:/jump".to_string());
        sounds.insert(SoundType::Land, "event:/land".to_string());

        Self {
            sounds,
            master_volume: 1.0,
            bgm: 1.0,
            sfx: 1.0,
            voice: 1.0,
            ui: 1.0,
            ambience: 1.0,
        }
    }
}

impl Config for SoundConfig {}
