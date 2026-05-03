use crate::entity_manager::EntityManager;
use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::parser::load_view_or_fallback;
use crate::ui::game_new::tree::UiTree;
use crate::ui::game_new::widgets::AbilitySlot;
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
}

impl GameHudView {
    pub fn new(_font_system: &mut FontSystem) -> Self {
        let tree = load_view_or_fallback("resources/ui/game_hud.ron");
        Self { tree }
    }

    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.tree.set_screen_size(width, height);
    }

    /// Layout, apply game data, drive hit-testing — call each frame while the HUD is visible.
    pub fn update(
        &mut self,
        font_system: &mut FontSystem,
        ctx: &mut UiContext,
        data: &PlayerHudData,
        portrait_texture_id: u32,
        em: &EntityManager,
        dt: f32,
    ) {
        self.tree.layout(font_system);

        if let Some(w) = self.tree.find_widget_mut("portrait") {
            if let Some(tr) = w.as_any_mut().downcast_mut::<TextureRect>() {
                tr.texture_id = portrait_texture_id;
            }
        }

        if let Some(w) = self.tree.find_widget_mut("player_name") {
            if let Some(lbl) = w.as_any_mut().downcast_mut::<Label>() {
                lbl.set_text(data.name.clone());
            }
        }

        if let Some(w) = self.tree.find_widget_mut("player_level") {
            if let Some(lbl) = w.as_any_mut().downcast_mut::<Label>() {
                lbl.set_text(format!("LV {}", data.level));
            }
        }

        if let Some(w) = self.tree.find_widget_mut("hp_bar") {
            if let Some(bar) = w.as_any_mut().downcast_mut::<ProgressBar>() {
                bar.set_value(data.health);
                bar.set_max_value(data.max_health);
            }
        }

        if let Some(w) = self.tree.find_widget_mut("mana_bar") {
            if let Some(bar) = w.as_any_mut().downcast_mut::<ProgressBar>() {
                bar.set_value(data.mana);
                bar.set_max_value(data.max_mana);
            }
        }

        self.sync_weapon_ability_slots(em, dt);

        let relayout_widgets = self.tree.update(ctx);
        if relayout_widgets {
            self.tree.force_layout();
        }
        self.tree.layout(font_system);
    }

    fn sync_weapon_ability_slots(&mut self, em: &EntityManager, dt: f32) {
        let slots = [
            ("slot_q", 2usize),
            ("slot_e", 3usize),
            ("slot_r", 5usize),
        ];
        let wa = em
            .player_main_hand_weapon()
            .and_then(|wid| em.weapon_abilities.get(wid));

        for (widget_id, slot_index) in slots {
            let (prog, ready, id_str, name, desc) = if let Some(w) = wa {
                let aid = match slot_index {
                    2 => w.q,
                    3 => w.e,
                    5 => w.r,
                    _ => 0,
                };
                let def = em.abilities_config.get(aid);
                (
                    w.get_cooldown_progress(slot_index, &em.abilities_config),
                    w.is_ready(slot_index),
                    format!("{}", aid),
                    def.map(|d| d.name.as_str()).unwrap_or(""),
                    def.map(|d| d.description.as_str()).unwrap_or(""),
                )
            } else {
                (0.0_f32, true, String::new(), "", "")
            };

            if let Some(widget) = self.tree.find_widget_mut(widget_id) {
                if let Some(slot) = widget.as_any_mut().downcast_mut::<AbilitySlot>() {
                    slot.set_data(0, prog, ready, &id_str, name, desc);
                    slot.update_glow_time(dt);
                }
            }
        }
    }
}
