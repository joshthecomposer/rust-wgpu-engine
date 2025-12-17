//! Player HUD view - handles HUD display logic.
//! Works with the GameRoot Slint component, which is owned by GameRootView.

use crate::entity_manager::EntityManager;

use super::game_root::GameRoot;

/// Context passed to PlayerHudView::update().
pub struct PlayerHudContext<'a> {
    pub entity_manager: &'a EntityManager,
    pub paused: bool,
}

/// Data extracted from entity manager for the player HUD.
pub struct PlayerHudData {
    pub visible: bool,
    pub name: String,
    pub level: u32,
    pub health: f32,
    pub max_health: f32,
    pub mana: f32,
    pub max_mana: f32,
}

impl PlayerHudData {
    pub fn from_entity_manager(em: &EntityManager) -> Self {
        if let Some(player_id) = em.get_player_id() {
            let name = em
                .names
                .get(player_id)
                .cloned()
                .unwrap_or_else(|| "PLAYER".to_string());
            let level = em.levels.get(player_id).copied().unwrap_or(1);
            let health = em.healths.get(player_id).copied().unwrap_or(100.0);
            let max_health = em.max_healths.get(player_id).copied().unwrap_or(100.0);
            let mana = em.manas.get(player_id).copied().unwrap_or(100.0);
            let max_mana = em.max_manas.get(player_id).copied().unwrap_or(100.0);

            Self {
                visible: true,
                name,
                level,
                health,
                max_health,
                mana,
                max_mana,
            }
        } else {
            Self {
                visible: false,
                name: "PLAYER".to_string(),
                level: 1,
                health: 100.0,
                max_health: 100.0,
                mana: 100.0,
                max_mana: 100.0,
            }
        }
    }
}

/// Manages the player HUD portion of the GameRoot component.
pub struct PlayerHudView;

impl PlayerHudView {
    /// Create a new PlayerHudView.
    pub fn new() -> Self {
        Self
    }

    /// Update the player HUD view.
    pub fn update(&self, game_root: &GameRoot, ctx: PlayerHudContext) {
        let hud_data = PlayerHudData::from_entity_manager(ctx.entity_manager);
        let show_hud = hud_data.visible && !ctx.paused;

        game_root.set_show_player_hud(show_hud);
        game_root.set_player_name(hud_data.name.into());
        game_root.set_player_level(hud_data.level as i32);
        game_root.set_player_health(hud_data.health);
        game_root.set_player_health_max(hud_data.max_health);
        game_root.set_player_mana(hud_data.mana);
        game_root.set_player_mana_max(hud_data.max_mana);
    }
}

impl Default for PlayerHudView {
    fn default() -> Self {
        Self::new()
    }
}
