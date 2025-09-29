use std::fs::{read_to_string, write};
use glam::{Quat, Vec3};
use toml::value::{Table, Value, Array};

use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};

use crate::enums_types::{EntityType, Faction};

#[derive(Deserialize, Debug, Serialize)]
pub struct WorldData {
    pub entities: Vec<EntityInstance>,
}

impl WorldData {
    pub fn load_from_file(file_name: &str) -> Self {
        println!("loading world data from {}", &file_name);
        let config_str = read_to_string(file_name).unwrap();

        toml::from_str(&config_str).expect("The world_data file was missing or malformed")
    }

    pub fn write_to_file(&self, file_name: &str) {
        println!("writing world data to {}", &file_name);

        let toml_str = toml::to_string_pretty(self).expect("Failed to deserialize world data");
        write(file_name, toml_str).expect("Failed to write world data");
    }

    pub fn write_readable_world_data(&self, path: &str) {
        // let mut root = Table::new();
        // let mut entities_array = Vec::new();

        // for entity in &self.entities {
        //     let mut ent = Table::new();
        //     ent.insert("entity_type".into(), Value::String(entity.entity_type.to_string()));
        //     ent.insert("faction".into(), Value::String(entity.faction.to_string()));

        //     let pos_array: Array = entity.position
        //         .iter()
        //         .map(|f| Value::Float(snap(*f as f64, 4)))  // 4 decimals
        //         .collect();
        //     ent.insert("position".into(), Value::Array(pos_array));

        //     let rot_array: Array = entity.rotation
        //         .iter()
        //         .map(|f| Value::Float(snap(*f as f64, 4)))  // 4 decimals
        //         .collect();
        //     ent.insert("rotation".into(), Value::Array(rot_array));

        //     entities_array.push(Value::Table(ent));
        // }

        // root.insert("entities".into(), Value::Array(entities_array));
        // let toml_str = toml::to_string(&Value::Table(root)).unwrap();
        // write(path, toml_str).unwrap();
    }
}

// =============================================================
// Helpers
// =============================================================
#[derive(Deserialize, Debug, Serialize)]
pub struct EntityInstance {
    pub entity_type: EntityType,
    pub faction: Faction,
    pub position: Vec3,
    pub rotation: Quat,
    pub weapons: Vec<EntityType>,
    pub base_speed: Option<f32>,
    pub health: f32,
}

fn snap(v: f64, precision: u32) -> f64 {
    let factor = 10f64.powi(precision as i32);
    (v * factor).round() / factor
}
