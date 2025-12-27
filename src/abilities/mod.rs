//! Weapon ability system - defines abilities and weapon ability pools.
//!
//! Each weapon type (sword, axe, etc.) has:
//! - Fixed M1/M2 basic attacks (same for all weapons of that type)
//! - Fixed Shift dodge ability
//! - Random Q/E skills from a pool
//! - Random R ultimate from a pool

use rand::prelude::{IndexedRandom, SliceRandom};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::Config;

/// Unique identifier for an ability.
pub type AbilityId = u32;

/// Definition of a single ability (loaded from config).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityDefinition {
    pub id: AbilityId,
    pub name: String,
    pub cooldown: f32,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub icon: String,
}

/// All ability definitions (loaded from abilities_config.json).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AbilitiesConfig {
    pub abilities: HashMap<String, AbilityDefinition>,
}

impl Config for AbilitiesConfig {}

impl AbilitiesConfig {
    /// Get an ability definition by ID.
    pub fn get(&self, id: AbilityId) -> Option<&AbilityDefinition> {
        self.abilities.get(&id.to_string())
    }

    /// Get the next available ability ID based on existing keys.
    pub fn get_next_id(&self) -> AbilityId {
        self.abilities
            .keys()
            .filter_map(|k| k.parse::<u32>().ok())
            .max()
            .unwrap_or(0)
            + 1
    }
}

/// Weapon type definition with fixed abilities and pools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponTypeAbilities {
    pub m1: AbilityId,
    pub m2: AbilityId,
    pub shift: AbilityId,
    pub skill_pool: Vec<AbilityId>,    // Q and E picked from here
    pub ultimate_pool: Vec<AbilityId>, // R picked from here
}

/// Configuration mapping weapon types to ability pools.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WeaponPoolsConfig {
    /// Weapon category definitions (e.g., "Sword", "Axe")
    pub weapon_types: HashMap<String, WeaponTypeAbilities>,
    /// Maps entity types to weapon categories (e.g., "OrcSword" -> "Sword")
    pub entity_type_mapping: HashMap<String, String>,
    /// Fixed abilities for the player's starting weapon
    pub starter_abilities: Option<StarterAbilities>,
}

impl Config for WeaponPoolsConfig {}

/// Fixed abilities for the starter weapon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarterAbilities {
    pub entity_type: String,
    pub q: AbilityId,
    pub e: AbilityId,
    pub r: AbilityId,
}

/// Runtime abilities assigned to a specific weapon instance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WeaponAbilities {
    pub m1: Option<AbilityId>,
    pub m2: Option<AbilityId>,
    pub q: Option<AbilityId>,
    pub e: Option<AbilityId>,
    pub shift: Option<AbilityId>,
    pub r: Option<AbilityId>,
    /// Current cooldowns for each slot (in seconds remaining).
    #[serde(skip)]
    pub cooldowns: [f32; 6],
}

impl WeaponAbilities {
    /// Get cooldown for a slot (index 0-5: M1, M2, Q, E, Shift, R).
    pub fn get_cooldown(&self, slot_index: usize) -> f32 {
        self.cooldowns.get(slot_index).copied().unwrap_or(0.0)
    }

    /// Get cooldown progress (0.0 = ready, 1.0 = just triggered) for UI display.
    pub fn get_cooldown_progress(
        &self,
        slot_index: usize,
        abilities_config: &AbilitiesConfig,
    ) -> f32 {
        let current = self.get_cooldown(slot_index);
        if current <= 0.0 {
            return 0.0;
        }

        // get the ability ID for this slot to find max cooldown
        let ability_id = match slot_index {
            0 => self.m1,
            1 => self.m2,
            2 => self.q,
            3 => self.e,
            4 => self.shift,
            5 => self.r,
            _ => None,
        };

        if let Some(id) = ability_id {
            if let Some(def) = abilities_config.get(id) {
                if def.cooldown > 0.0 {
                    return current / def.cooldown;
                }
            }
        }

        0.0
    }

    /// Check if a slot is ready (off cooldown).
    pub fn is_ready(&self, slot_index: usize) -> bool {
        self.get_cooldown(slot_index) <= 0.0
    }

    /// Trigger an ability, starting its cooldown.
    /// Returns the ability ID if successful, None if on cooldown or slot empty.
    pub fn trigger(
        &mut self,
        slot_index: usize,
        abilities_config: &AbilitiesConfig,
    ) -> Option<AbilityId> {
        if !self.is_ready(slot_index) {
            return None;
        }

        let ability_id = match slot_index {
            0 => self.m1,
            1 => self.m2,
            2 => self.q,
            3 => self.e,
            4 => self.shift,
            5 => self.r,
            _ => None,
        }?;

        // start the cooldown
        if let Some(def) = abilities_config.get(ability_id) {
            if slot_index < 6 {
                self.cooldowns[slot_index] = def.cooldown;
            }
        }

        Some(ability_id)
    }

    /// Update cooldowns by delta time (call each frame).
    pub fn tick(&mut self, dt: f32) {
        for cd in &mut self.cooldowns {
            if *cd > 0.0 {
                *cd = (*cd - dt).max(0.0);
            }
        }
    }

    /// Generate abilities for a weapon based on its entity type.
    /// Returns None if the entity type isn't a known weapon.
    pub fn generate<R: Rng>(
        entity_type: &str,
        pools_config: &WeaponPoolsConfig,
        rng: &mut R,
        is_starter: bool,
    ) -> Option<Self> {
        // find the weapon category for this entity type
        let category = pools_config.entity_type_mapping.get(entity_type)?;
        let weapon_type = pools_config.weapon_types.get(category)?;

        // check if this is the starter weapon with fixed abilities
        if is_starter {
            if let Some(starter) = &pools_config.starter_abilities {
                if starter.entity_type == entity_type {
                    return Some(Self {
                        m1: Some(weapon_type.m1),
                        m2: Some(weapon_type.m2),
                        q: Some(starter.q),
                        e: Some(starter.e),
                        shift: Some(weapon_type.shift),
                        r: Some(starter.r),
                        cooldowns: [0.0; 6],
                    });
                }
            }
        }

        // randomly pick Q and E from skill pool (must be different)
        let mut skill_pool = weapon_type.skill_pool.clone();
        skill_pool.shuffle(rng);
        let q = skill_pool.get(0).copied();
        let e = skill_pool.get(1).copied();

        // randomly pick R from ultimate pool
        let r = weapon_type.ultimate_pool.choose(rng).copied();

        Some(Self {
            m1: Some(weapon_type.m1),
            m2: Some(weapon_type.m2),
            q,
            e,
            shift: Some(weapon_type.shift),
            r,
            cooldowns: [0.0; 6],
        })
    }
}
