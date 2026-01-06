use crate::entity_manager::EntityManager;
use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::parser::load_view_or_fallback;
use crate::ui::game_new::tree::UiTree;
use crate::ui::game_new::widgets::Label;
use crate::ui::game_new::widgets::ProgressBar;
use crate::ui::game_new::widgets::TextureRect;

/// Data for the HUD.
#[derive(PartialEq, Clone, Debug)]
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

pub struct GameHudView {
    pub tree: UiTree,
    needs_layout: bool,
}

impl GameHudView {
    pub fn new(_font_system: &mut FontSystem) -> Self {
        let tree = load_view_or_fallback("resources/ui/game_hud.ron");
        Self {
            tree,
            needs_layout: true,
        }
    }

    pub fn update(&mut self, ctx: &mut UiContext, data: &PlayerHudData, portrait_texture_id: u32) {
        if !data.visible {
            // TODO: Hide the view or not update?
            // If we have a root widget, we could hide it or just not update/render?
            // UiTree render checks specific things?
            // For now, let's assume we update anyway or handle visibility in tree.
            // But we can early return if we want constant state, but we should update values.
        }

        // Simpler approach: Always request layout update when data comes in for now.
        // Optimization: Only set if data actually changed.
        self.needs_layout = true;

        self.tree.update(ctx);

        // update Portrait
        if let Some(w) = self.tree.find_widget_mut("portrait") {
            if let Some(tr) = w.as_any_mut().downcast_mut::<TextureRect>() {
                tr.texture_id = portrait_texture_id;
            }
        }

        // update Name
        if let Some(w) = self.tree.find_widget_mut("player_name") {
            if let Some(lbl) = w.as_any_mut().downcast_mut::<Label>() {
                lbl.set_text(data.name.clone());
            }
        }

        // update Level
        if let Some(w) = self.tree.find_widget_mut("player_level") {
            if let Some(lbl) = w.as_any_mut().downcast_mut::<Label>() {
                lbl.set_text(format!("LV {}", data.level));
            }
        }

        // update HP Bar
        if let Some(w) = self.tree.find_widget_mut("hp_bar") {
            if let Some(bar) = w.as_any_mut().downcast_mut::<ProgressBar>() {
                bar.set_value(data.health);
                bar.set_max_value(data.max_health);
            }
        }

        // update Mana Bar
        if let Some(w) = self.tree.find_widget_mut("mana_bar") {
            if let Some(bar) = w.as_any_mut().downcast_mut::<ProgressBar>() {
                bar.set_value(data.mana);
                bar.set_max_value(data.max_mana);
            }
        }
    }

    pub fn needs_render(&self) -> bool {
        self.needs_layout
    }

    pub fn clear_render_flag(&mut self) {
        self.needs_layout = false;
    }
}
