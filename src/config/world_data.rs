use std::fs::{read_to_string, write};
use glam::{Quat, Vec3};
use toml::value::{Table, Value, Array};

use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};

use crate::enums_types::{EntityType, Faction};

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct WorldData {
    pub entities: Vec<EntityInstance>,
}

impl WorldData {
    pub fn load_from_file(file_name: &str) -> Self {
        println!("loading world data from {}", &file_name);
        let config_str = read_to_string(file_name).unwrap();

        serde_json::from_str(&config_str).expect("The world_data file was missing or malformed")
    }

    pub fn write_to_file(&self, file_name: &str) {
        println!("writing world data to {}", &file_name);

        let json_string = serde_json::to_string_pretty(self).expect("Failed to serialize world data");
        write(file_name, json_string).expect("Failed to write world data");

        println!("Completed writing world data to {}", &file_name);
    }
}

// =============================================================
// Helpers
// =============================================================
#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct EntityInstance {
    pub entity_type: String,
    pub faction: String,
    pub position: Vec3,
    pub rotation: Quat,
    #[serde(default)]
    pub weapons: Option<Vec<EntityInstance>>,
    #[serde(default)]
    pub base_speed: Option<f32>,
    #[serde(default)]
    pub health: Option<f32>,
    #[serde(default)]
    pub jump_height: Option<f32>,
    #[serde(default)]
    pub cleanup_timer: Option<f32>,
}

fn snap(v: f64, precision: u32) -> f64 {
    let factor = 10f64.powi(precision as i32);
    (v * factor).round() / factor
}
