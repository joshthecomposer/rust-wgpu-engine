use std::{collections::HashMap, fs::read_to_string};

use glam::{Quat, Vec3};
use serde::Deserialize;

use crate::{debug::gizmos::Cylinder, enums_types::{AnimationType, EntityType, Faction, HitboxShape, SoundType}};

#[derive(Clone, Deserialize, Debug)]
pub struct ItemBones {
    pub rh_name: String,
    pub lh_name: String,
}

#[derive(Deserialize, Debug)]
pub struct EntityConfig {
    pub entity_types: HashMap<EntityType, EntityTypeHelper>
}

impl EntityConfig {
    pub fn load_from_file(file_name: &str) -> EntityConfig {
        println!("loading entity configuration from {}", &file_name);
        let config_str = read_to_string(file_name).unwrap();

        serde_json::from_str(&config_str).expect("The entity config file was missing")
    }
}


// =============================================================
// Helpers
// =============================================================

#[derive(Deserialize, Debug)]
pub struct AnimationPropHelper {
    pub name: AnimationType,
    pub one_shots: HashMap<SoundType, Vec<u32>>,
    pub continuous_sounds: Vec<SoundType>,
    pub hurtbox_activation: Vec<u32>,
    #[serde(default)]
    pub hold_frame: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct EntityTypeHelper {
    pub rot_correction: Quat,
    pub scale_correction: Vec3,
    pub mesh_path: String,
    #[serde(default)]
    pub bone_path: Option<String>,
    #[serde(default)]
    pub animation_properties: Option<Vec<AnimationPropHelper>>,
    pub item_bones: ItemBones,
    pub aggro_range: f32,
    pub hitbox: HitboxShape,
}
