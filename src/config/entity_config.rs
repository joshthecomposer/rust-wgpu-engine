use std::{
    collections::HashMap,
    fs::{read_to_string, write},
};

use glam::{Quat, Vec3};
use rapier3d::prelude::ColliderType;
use serde::{Deserialize, Serialize};

use crate::{
    debug::gizmos::Cylinder,
    enums_types::{AnimationType, EntityType, Faction, HitboxShape, SoundType},
};

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct ItemBones {
    pub rh: usize,
    pub lh: usize,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct EntityConfig {
    pub entity_types: HashMap<String, EntityTypeHelper>,
}

impl EntityConfig {
    pub fn load_from_file(file_name: &str) -> EntityConfig {
        println!("loading entity configuration from {}", file_name);
        let config_str = read_to_string(file_name).unwrap();

        serde_json::from_str(&config_str).expect("The entity config file was missing")
    }

    pub fn write_to_file(&self, file_name: &str) {
        println!("writing entity type data to {}", file_name);

        let json_string =
            serde_json::to_string_pretty(self).expect("Failed to serialize entity type data");
        write(file_name, json_string).expect("Failed to write entity type file");

        println!("Completed writing entity type data to {}", file_name);
    }
}

// =============================================================
// Helpers
// =============================================================

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct ItemBoneNames {
    pub rh: String,
    pub lh: String,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct AnimationPropHelper {
    pub name: AnimationType,
    pub one_shots: HashMap<SoundType, Vec<u32>>,
    pub continuous_sounds: Vec<SoundType>,
    pub hurtbox_activation: Vec<u32>,
    #[serde(default)]
    pub hold_frame: Option<u32>,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct EntityTypeHelper {
    pub rot_correction: Quat,
    pub scale_correction: Vec3,
    pub mesh_path: String,
    #[serde(default)]
    pub bone_path: Option<String>,
    #[serde(default)]
    pub animation_properties: Option<Vec<AnimationPropHelper>>,
    #[serde(default)]
    pub item_bones: Option<ItemBoneNames>,
    pub aggro_range: Option<f32>,
    pub hitbox: HitboxShape,
    #[serde(default)]
    pub total_mass: Option<f32>,
    #[serde(default)]
    pub controller_type: Option<ColliderType>,
}

impl Default for EntityTypeHelper {
    fn default() -> Self {
        Self {
            rot_correction: Quat::IDENTITY,
            scale_correction: Vec3::ONE,
            mesh_path: String::new(),
            bone_path: None,
            animation_properties: None,
            item_bones: None,
            aggro_range: None,
            hitbox: HitboxShape::BoundingBox,
            total_mass: None,
            controller_type: None,
        }
    }
}

impl EntityTypeHelper {
    pub fn from_ui_helper(ui_helper: &UiEntityTypeHelper) -> Self {
        Self {
            rot_correction: Quat::from_array(ui_helper.rot_correction),
            scale_correction: ui_helper.scale_correction.into(),
            mesh_path: ui_helper.mesh_path.clone(),
            bone_path: None,
            animation_properties: None,
            item_bones: None,
            aggro_range: if ui_helper.aggro_range > 0.0 {
                Some(ui_helper.aggro_range)
            } else {
                None
            },
            hitbox: HitboxShape::BoundingBox,
            total_mass: None,
            controller_type: None,
        }
    }
}

// This is currently for ImGUI saving a new entity type.
#[derive(Deserialize, Debug)]
pub struct UiEntityTypeHelper {
    pub entity_type: String,
    pub rot_correction: [f32; 4],
    pub scale_correction: [f32; 3],
    pub mesh_path: String,
    pub aggro_range: f32,
    pub hitbox: String,
    pub texture_path: String,
    pub total_mass: f32,

    pub r: f32,
    pub h: f32,
    pub hx: f32,
    pub hy: f32,
    pub hz: f32,
}

impl Default for UiEntityTypeHelper {
    fn default() -> Self {
        Self {
            entity_type: String::new(),
            rot_correction: [0.0, 0.0, 0.0, 1.0],
            scale_correction: [1.0, 1.0, 1.0],
            mesh_path: String::new(),
            aggro_range: 0.0,
            hitbox: String::new(),
            texture_path: String::new(),
            total_mass: 0.0,

            r: 0.0,
            h: 0.0,
            hx: 0.0,
            hy: 0.0,
            hz: 0.0,
        }
    }
}
