//! Ability bar view - displays weapon abilities on the HUD.
//! Works with the GameRoot Slint component, which is owned by GameRootView.

use crate::abilities::{AbilitiesConfig, WeaponAbilities};
use crate::entity_manager::EntityManager;
use crate::ui::image_cache::UiImageCache;

use super::game_root::AbilitySlotData;

/// Context passed to AbilityBarView::update().
pub struct AbilityBarContext<'a> {
    pub entity_manager: &'a EntityManager,
    pub paused: bool,
    pub image_cache: &'a mut UiImageCache,
    pub elapsed_time: f64,
}

/// Data for a single ability slot.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SlotDisplayData {
    pub visible: bool,
    pub icon_path: String,
    pub key_label: String,
    pub cooldown_progress: f32,
    pub cooldown_time_remaining: f32,
    pub is_ready: bool,
    pub ability_id: String,
    pub ability_name: String,
    pub ability_description: String,
    pub ability_effects: String,
}

impl SlotDisplayData {
    pub fn to_slint(&self, image_cache: &mut UiImageCache) -> AbilitySlotData {
        AbilitySlotData {
            visible: self.visible,
            icon: image_cache.get(&self.icon_path),
            key_label: self.key_label.clone().into(),
            cooldown_progress: self.cooldown_progress,
            cooldown_time_remaining: self.cooldown_time_remaining,
            is_ready: self.is_ready,
            ability_id: self.ability_id.clone().into(),
            ability_name: self.ability_name.clone().into(),
            ability_description: self.ability_description.clone().into(),
            ability_effects: self.ability_effects.clone().into(),
        }
    }
}

/// Data extracted from entity manager for the ability bar.
#[derive(Debug, Clone, Default)]
pub struct AbilityBarData {
    pub visible: bool,
    pub q: SlotDisplayData,
    pub e: SlotDisplayData,
    pub r: SlotDisplayData,
}

impl AbilityBarData {
    /// Extract ability bar data from the player's equipped weapon.
    pub fn from_entity_manager(em: &EntityManager) -> Self {
        let player_id = match em.get_player_id() {
            Some(id) => id,
            None => return Self::default(),
        };

        let equipped_weapon_id = match em.active_items.get(player_id) {
            Some(active) => match active.right_hand {
                Some(id) => id,
                None => return Self::default(),
            },
            None => return Self::default(),
        };

        let weapon_abilities = match em.weapon_abilities.get(equipped_weapon_id) {
            Some(a) => a,
            None => return Self::default(),
        };

        Self::from_weapon_abilities(weapon_abilities, &em.abilities_config)
    }

    /// Create ability bar data from weapon abilities and config.
    ///
    /// Builds display data for all six ability slots (M1, M2, Q, E, Shift, R)
    /// using the weapon's assigned abilities and their cooldown states.
    fn from_weapon_abilities(abilities: &WeaponAbilities, config: &AbilitiesConfig) -> Self {
        Self {
            visible: true,
            q: Self::make_slot(abilities, 2, "Q", config),
            e: Self::make_slot(abilities, 3, "E", config),
            r: Self::make_slot(abilities, 5, "R", config),
        }
    }

    /// Create display data for a single ability slot.
    ///
    /// # Arguments
    ///
    /// * `abilities` - The weapon's ability assignments and cooldown states
    /// * `slot_index` - The slot index (0-5 for M1, M2, Q, E, Shift, R)
    /// * `key_label` - The display label for the keybind (e.g., "M1", "Q")
    /// * `config` - The abilities configuration for looking up cooldown durations
    ///
    /// # Returns
    ///
    /// A `SlotDisplayData` with visibility, ability ID, cooldown progress, and ready state.
    /// If the slot has no ability assigned, returns a hidden slot with default values.
    fn make_slot(
        abilities: &WeaponAbilities,
        slot_index: usize,
        key_label: &str,
        config: &AbilitiesConfig,
    ) -> SlotDisplayData {
        let ability_id = match slot_index {
            2 => abilities.q,
            3 => abilities.e,
            5 => abilities.r,
            _ => panic!("cannot be!"),
        };

        let progress = abilities.get_cooldown_progress(slot_index, config);
        let time_remaining = abilities.get_cooldown(slot_index);

        // fetch ability definition for tooltip data
        let (name, description) = match config.get(ability_id) {
            Some(def) => (def.name.clone(), def.description.clone()),
            None => (String::new(), String::new()),
        };

        SlotDisplayData {
            visible: true,
            icon_path: match config.get(ability_id) {
                Some(def) => def.icon.clone(),
                None => String::new(),
            },
            key_label: key_label.to_string(),
            cooldown_progress: progress,
            cooldown_time_remaining: time_remaining,
            is_ready: abilities.is_ready(slot_index),
            ability_id: ability_id.to_string(),
            ability_name: name,
            ability_description: description,
            ability_effects: String::new(), // empty for now, ready for future
        }
    }
}

/// Manages the ability bar portion of the GameRoot component.
pub struct AbilityBarView;

impl AbilityBarView {
    /// Create a new AbilityBarView.
    pub fn new() -> Self {
        Self
    }

    /// Update the ability bar view.
    /// Now works with AbilityBarRenderer instead of GameRoot for optimized rendering.
    pub fn update(
        &self,
        renderer: &mut crate::ui::ability_bar_renderer::AbilityBarRenderer,
        ctx: AbilityBarContext,
    ) {
        let data = AbilityBarData::from_entity_manager(ctx.entity_manager);
        let show = data.visible && !ctx.paused;

        // build the slots array for the renderer
        let slots = [data.q, data.e, data.r];

        // update the renderer with throttling and change detection
        renderer.update(show, slots, ctx.image_cache, ctx.elapsed_time);
    }
}

impl Default for AbilityBarView {
    fn default() -> Self {
        Self::new()
    }
}
