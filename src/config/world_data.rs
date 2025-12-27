use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct WorldData {
    pub entities: Vec<EntityInstance>,
}

impl Default for WorldData {
    fn default() -> Self {
        Self {
            entities: Vec::new(),
        }
    }
}

impl Config for WorldData {}

// =============================================================
// Helpers
// =============================================================
#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct EntityInstance {
    pub entity_type: String,
    #[serde(default)]
    pub faction: Option<String>,
    pub position: Vec3,
    pub rotation: Quat,
    #[serde(default)]
    pub weapons: Option<Vec<EntityInstance>>,
    #[serde(default)]
    pub base_speed: Option<f32>,
    #[serde(default)]
    pub health: Option<f32>,
    #[serde(default)]
    pub max_health: Option<f32>,
    #[serde(default)]
    pub mana: Option<f32>,
    #[serde(default)]
    pub max_mana: Option<f32>,
    #[serde(default)]
    pub level: Option<u32>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub jump_height: Option<f32>,
    #[serde(default)]
    pub cleanup_timer: Option<f32>,
    #[serde(default)]
    pub pickup_range: Option<f32>,
}

// fn snap(v: f64, precision: u32) -> f64 {
//     let factor = 10f64.powi(precision as i32);
//     (v * factor).round() / factor
// }
