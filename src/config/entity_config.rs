use std::{collections::HashMap, fs::read_to_string};

use serde::Deserialize;

use crate::{debug::gizmos::Cylinder, enums_types::{AnimationType, EntityType, Faction, SoundType}};

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
}

#[derive(Deserialize, Debug)]
pub struct EntityTypeHelper {
    pub rot_correction: String,
    pub scale_correction: [f32; 3],
    pub mesh_path: String,
    pub bone_path: String,
    pub hit_cyl: Option<Cylinder>,
    pub animation_properties: Vec<AnimationPropHelper>,
    pub flip_180: bool,
    pub item_bones: ItemBones,
}
