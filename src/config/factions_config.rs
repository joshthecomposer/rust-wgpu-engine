use std::{collections::{HashMap, HashSet}, fs::{read_to_string, write}};

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::{debug::gizmos::Cylinder, enums_types::{AnimationType, EntityType, Faction, HitboxShape, SoundType}};

#[derive(Deserialize, Debug, Serialize)]
pub struct FactionsConfig {
    pub factions: HashSet<String>
}

impl FactionsConfig {
    pub fn load_from_file(file_name: &str) -> Self {
        println!("loading faction configuration from {}", file_name);
        let config_str = read_to_string(file_name).unwrap();

        serde_json::from_str(&config_str).expect("The faction config file was missing")
    }

    pub fn write_to_file(&self, file_name: &str) {
        println!("writing faction data to {}", file_name);

        let json_string = serde_json::to_string_pretty(self).expect("Failed to serialize faction data");
        write(file_name, json_string).expect("Failed to write faction file");

        println!("Completed writing faction data to {}", file_name);
    }
}


// =============================================================
// Helpers
// =============================================================

