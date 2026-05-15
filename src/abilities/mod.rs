use rand::prelude::{IndexedRandom, SliceRandom};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::Config;
use crate::enums_types::DamagePayload;

pub type AbilityId = usize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityDefinition {
    pub id: AbilityId,
    pub name: String,
    pub cooldown: f32,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub icon: String,
    // this is the same animation name that is stored in the animator
    pub animation: String,
    #[serde(default)]
    pub payload: Option<DamagePayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AbilitiesConfig {
    pub abilities: Vec<AbilityDefinition>,
}

impl Config for AbilitiesConfig {}

impl AbilitiesConfig {
    pub fn get(&self, id: AbilityId) -> Option<&AbilityDefinition> {
        self.abilities.iter().find(|a| a.id == id)
    }

    pub fn get_next_id(&self) -> usize {
        self.abilities
            .iter()
            .filter_map(|a| Some(a.id))
            .max()
            .unwrap_or(0)
    }

    pub fn remove_by_id_unordered(&mut self, id: usize) -> Option<AbilityDefinition> {
        let i = self.abilities.iter().position(|a| a.id == id)?;
        Some(self.abilities.swap_remove(i))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponTypeAbilities {
    pub skill_pool: Vec<AbilityId>,    // Q and E picked from here
    pub ultimate_pool: Vec<AbilityId>, // R picked from here
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WeaponPoolsConfig {
    pub weapon_types: HashMap<String, WeaponTypeAbilities>,
}

impl Config for WeaponPoolsConfig {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WeaponAbilities {
    pub q: AbilityId,
    pub e: AbilityId,
    pub r: AbilityId,
    /// Current cooldowns for each slot (in seconds remaining).
    #[serde(skip)]
    pub cooldowns: [f32; 6],
}

impl WeaponAbilities {
    pub fn get_cooldown(&self, slot_index: usize) -> f32 {
        self.cooldowns.get(slot_index).copied().unwrap_or(0.0)
    }

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
            2 => self.q,
            3 => self.e,
            5 => self.r,
            _ => panic!("this simply cannot be!"),
        };

        if let Some(def) = abilities_config.get(ability_id) {
            if def.cooldown > 0.0 {
                return current / def.cooldown;
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
            2 => self.q,
            3 => self.e,
            5 => self.r,
            _ => panic!("ahh fuck, I can't believe you've done this"),
        };

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
    ) -> Self {
        let pool_config = match pools_config.weapon_types.get(entity_type) {
            Some(pc) => pc,
            None => panic!("We can't find a pool config for type: {}", entity_type),
        };

        // randomly pick skills from pools (Q and E are deduped)
        let mut skill_pool = pool_config.skill_pool.clone();
        skill_pool.shuffle(rng);
        let q = skill_pool.get(0).copied().unwrap();
        let e = skill_pool.get(1).copied().unwrap();
        let r = pool_config.ultimate_pool.choose(rng).copied().unwrap();

        Self {
            q,
            e,
            r,
            cooldowns: [0.0; 6],
        }
    }
}
