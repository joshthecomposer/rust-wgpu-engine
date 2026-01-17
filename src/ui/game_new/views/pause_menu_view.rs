//! Pause menu view for the new GPU UI system.
//!
//! Replaces the Slint-based pause menu with custom GPU-rendered UI.
//! Handles system actions (reload, save, quit) and settings (display, interface, sound).

use crate::config::game_config::GameConfig;
use crate::config::sound_config::SoundConfig;
use crate::entity_manager::EntityManager;
use crate::toast;
use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::parser::theme::load_theme;
use crate::ui::game_new::parser::{load_view, theme::Theme};
use crate::ui::game_new::styles::Color;
use crate::ui::game_new::tree::UiTree;
use crate::ui::game_new::widgets::{Checkbox, CloseButton, ComboBox, MenuButton, Slider, TabView};
use crate::ui::message_queue::{MessageQueue, UiMessage};

/// Context passed to PauseMenuView::update().
pub struct PauseMenuUpdateContext<'a> {
    pub paused: &'a mut bool,
    pub render_gizmos: &'a mut bool,
    pub game_config: &'a mut GameConfig,
    pub sound_config: &'a mut SoundConfig,
    pub entity_manager: &'a EntityManager,
    pub message_queue: &'a mut MessageQueue,
    pub input_state: &'a crate::input::InputState,
}

/// Manages the pause menu UI using the new GPU rendering system.
pub struct PauseMenuView {
    pub tree: UiTree,
    needs_layout: bool,
    settings_initialized: bool,
    active_tab: usize,
    theme: Theme,
}

impl PauseMenuView {
    /// Create a new pause menu view by loading from RON.
    pub fn new(font_system: &mut FontSystem) -> Self {
        let tree = match load_view("resources/ui/pause_menu.ron") {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Failed to load pause_menu.ron: {}", e);
                UiTree::new()
            }
        };

        let theme = load_theme("resources/ui/theme.ron").unwrap_or_else(|e| {
            eprintln!("Failed to load theme: {}", e);
            Theme::new()
        });

        let mut view = Self {
            tree,
            needs_layout: true,
            settings_initialized: false,
            active_tab: 0,
            theme,
        };

        view.tree.layout(font_system);

        view
    }

    /// Resolve a color variable using the theme.
    fn resolve_color(&self, color: Color) -> Color {
        if let Color::Variable(name) = &color {
            self.theme.get_color(name).unwrap_or(color)
        } else {
            color
        }
    }

    /// Update the pause menu, handling button clicks and syncing settings.
    pub fn update(
        &mut self,
        update_ctx: &mut PauseMenuUpdateContext,
        font_system: &mut FontSystem,
    ) {
        let mut ui_ctx = UiContext {
            input: update_ctx.input_state,
            messages: update_ctx.message_queue,
        };
        let needs_relayout = self.tree.update(&mut ui_ctx);

        self.update_tab_state();
        self.update_panel_height();
        self.handle_button_clicks(update_ctx);
        self.sync_settings_from_ui(update_ctx);

        // ! CRITICAL: Relayout if tree.update() returned true (e.g., ScrollView scrolled)
        // ! or if we manually set needs_layout (e.g., tab changed, panel height changed)
        if needs_relayout || self.needs_layout {
            self.tree.force_layout();
            self.tree.layout(font_system);
            self.needs_layout = false;
        }
    }

    /// Update tab state tracking.
    fn update_tab_state(&mut self) {
        let current_tab = if let Some(widget) = self.tree.find_widget_mut("main_tabs") {
            if let Some(tab_view) = widget.as_any_mut().downcast_mut::<TabView>() {
                Some(tab_view.selected_index())
            } else {
                None
            }
        } else {
            None
        };

        if let Some(tab) = current_tab {
            if tab != self.active_tab {
                self.active_tab = tab;
                self.needs_layout = true;
                // ! CRITICAL: Also force tree's internal needs_layout flag
                // ! so UiTree.layout() doesn't early-return
                self.tree.force_layout();
            }
        }
    }

    /// Update panel height based on active tab.
    fn update_panel_height(&mut self) {
        // TODO: could optimize by caching current height and only updating if changed
        let target_height = if self.active_tab == 0 {
            380.0 // SYSTEM tab (smaller, just menu buttons)
        } else {
            487.0 // SETTINGS tab: 16 (pad) + 20 (close) + 36 (title) + 11 (accent) + 388 (tabview) + 16 (pad) = 487
        };

        if let Some(widget) = self.tree.find_widget_mut("pause_menu_panel") {
            if let Some(col) = widget
                .as_any_mut()
                .downcast_mut::<crate::ui::game_new::widgets::Column>()
            {
                if let Some(current_height) = col.style.height.resolve(1000.0) {
                    if (current_height - target_height).abs() > 1.0 {
                        col.style.height = crate::ui::game_new::styles::Length::Px(target_height);
                        // update margin_top to overlap shadow by 4px
                        // shadow is at margin_top: 4px, panel should overlap it by moving up 4px
                        col.style.margin_top = Some(crate::ui::game_new::styles::Length::Px(-4.0));
                        self.needs_layout = true;
                    }
                } else {
                    col.style.height = crate::ui::game_new::styles::Length::Px(target_height);
                    col.style.margin_top = Some(crate::ui::game_new::styles::Length::Px(-4.0));
                    self.needs_layout = true;
                }
            }
        }

        // Update shadow height to match panel
        if let Some(widget) = self.tree.find_widget_mut("pause_menu_shadow") {
            if let Some(box_widget) = widget
                .as_any_mut()
                .downcast_mut::<crate::ui::game_new::widgets::BoxWidget>()
            {
                if let Some(current_height) = box_widget.style.height.resolve(1000.0) {
                    if (current_height - target_height).abs() > 1.0 {
                        box_widget.style.height =
                            crate::ui::game_new::styles::Length::Px(target_height);
                        self.needs_layout = true;
                    }
                } else {
                    box_widget.style.height =
                        crate::ui::game_new::styles::Length::Px(target_height);
                    self.needs_layout = true;
                }
            }
        }
    }

    /// Handle button click events by checking widget IDs.
    fn handle_button_clicks(&mut self, ctx: &mut PauseMenuUpdateContext) {
        if let Some(widget) = self.tree.find_widget_mut("btn_close") {
            if let Some(close_btn) = widget.as_any_mut().downcast_mut::<CloseButton>() {
                if close_btn.is_pressed() {
                    *ctx.paused = false;
                }
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("btn_reload_world") {
            if let Some(btn) = widget.as_any_mut().downcast_mut::<MenuButton>() {
                if btn.is_pressed() {
                    ctx.message_queue.send(UiMessage::ReloadWorldData);
                }
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("btn_save_player") {
            if let Some(btn) = widget.as_any_mut().downcast_mut::<MenuButton>() {
                if btn.is_pressed() {
                    ctx.entity_manager
                        .serialize_entity_data("config/player_data.json");
                    toast!(
                        Success,
                        "Player Data Saved",
                        "Your player data has been saved successfully."
                    );
                }
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("btn_quit") {
            if let Some(btn) = widget.as_any_mut().downcast_mut::<MenuButton>() {
                if btn.is_pressed() {
                    ctx.message_queue.send(UiMessage::WindowShouldClose);
                }
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("btn_cancel") {
            if let Some(btn) = widget.as_any_mut().downcast_mut::<MenuButton>() {
                if btn.is_pressed() {
                    *ctx.paused = false;
                    ctx.message_queue.send(UiMessage::CancelSettings);
                    self.settings_initialized = false;
                }
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("btn_apply") {
            if let Some(btn) = widget.as_any_mut().downcast_mut::<MenuButton>() {
                if btn.is_pressed() {
                    ctx.message_queue.send(UiMessage::ApplySettings);
                }
            }
        }
    }

    /// Sync settings from UI widgets to game config.
    fn sync_settings_from_ui(&mut self, ctx: &mut PauseMenuUpdateContext) {
        if !self.settings_initialized {
            self.init_settings_from_config(ctx);
            self.settings_initialized = true;
            return;
        }

        if let Some(widget) = self.tree.find_widget_mut("combo_resolution") {
            if let Some(combo) = widget.as_any().downcast_ref::<ComboBox>() {
                let selected = combo.selected_text();
                if let Some((w, h)) = parse_resolution(selected) {
                    ctx.game_config.win_width = w;
                    ctx.game_config.win_height = h;
                }
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("combo_window_mode") {
            if let Some(combo) = widget.as_any().downcast_ref::<ComboBox>() {
                ctx.game_config.window_mode = combo.selected_text().to_string();
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("check_vsync") {
            if let Some(checkbox) = widget.as_any().downcast_ref::<Checkbox>() {
                ctx.game_config.vsync = checkbox.is_checked();
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("combo_msaa") {
            if let Some(combo) = widget.as_any().downcast_ref::<ComboBox>() {
                ctx.game_config.msaa_level = parse_msaa(combo.selected_text());
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("check_gizmos") {
            if let Some(checkbox) = widget.as_any().downcast_ref::<Checkbox>() {
                *ctx.render_gizmos = checkbox.is_checked();
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("check_fps") {
            if let Some(checkbox) = widget.as_any().downcast_ref::<Checkbox>() {
                ctx.game_config.fps_counter = checkbox.is_checked();
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("check_debug") {
            if let Some(checkbox) = widget.as_any().downcast_ref::<Checkbox>() {
                ctx.game_config.debug_mode = checkbox.is_checked();
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("combo_font") {
            if let Some(combo) = widget.as_any().downcast_ref::<ComboBox>() {
                let ui_font_name = combo.selected_text();
                let font_family = ui_font_to_family(ui_font_name);
                ctx.game_config.font_family = font_family;
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("slider_bgm") {
            if let Some(slider) = widget.as_any().downcast_ref::<Slider>() {
                ctx.sound_config.bgm = slider.value();
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("slider_sfx") {
            if let Some(slider) = widget.as_any().downcast_ref::<Slider>() {
                ctx.sound_config.sfx = slider.value();
            }
        }
    }

    /// Initialize UI widgets from config values.
    fn init_settings_from_config(&mut self, ctx: &PauseMenuUpdateContext) {
        if let Some(widget) = self.tree.find_widget_mut("combo_resolution") {
            if let Some(combo) = widget.as_any_mut().downcast_mut::<ComboBox>() {
                let res_str = format!(
                    "{} x {}",
                    ctx.game_config.win_width, ctx.game_config.win_height
                );
                for (i, opt) in combo.options.iter().enumerate() {
                    if opt == &res_str {
                        combo.set_selected_index(i);
                        break;
                    }
                }
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("combo_window_mode") {
            if let Some(combo) = widget.as_any_mut().downcast_mut::<ComboBox>() {
                for (i, opt) in combo.options.iter().enumerate() {
                    if opt == &ctx.game_config.window_mode {
                        combo.set_selected_index(i);
                        break;
                    }
                }
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("check_vsync") {
            if let Some(checkbox) = widget.as_any_mut().downcast_mut::<Checkbox>() {
                checkbox.set_checked(ctx.game_config.vsync);
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("combo_msaa") {
            if let Some(combo) = widget.as_any_mut().downcast_mut::<ComboBox>() {
                let idx = match ctx.game_config.msaa_level {
                    1 => 0,  // None
                    2 => 1,  // 2x MSAA
                    4 => 2,  // 4x MSAA
                    8 => 3,  // 8x MSAA
                    16 => 4, // 16x MSAA
                    _ => 4,
                };
                combo.set_selected_index(idx);
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("check_gizmos") {
            if let Some(checkbox) = widget.as_any_mut().downcast_mut::<Checkbox>() {
                checkbox.set_checked(*ctx.render_gizmos);
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("check_fps") {
            if let Some(checkbox) = widget.as_any_mut().downcast_mut::<Checkbox>() {
                checkbox.set_checked(ctx.game_config.fps_counter);
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("check_debug") {
            if let Some(checkbox) = widget.as_any_mut().downcast_mut::<Checkbox>() {
                checkbox.set_checked(ctx.game_config.debug_mode);
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("combo_font") {
            if let Some(combo) = widget.as_any_mut().downcast_mut::<ComboBox>() {
                let ui_font_name = family_to_ui_font(&ctx.game_config.font_family);
                for (i, opt) in combo.options.iter().enumerate() {
                    if opt == &ui_font_name {
                        combo.set_selected_index(i);
                        break;
                    }
                }
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("slider_bgm") {
            if let Some(slider) = widget.as_any_mut().downcast_mut::<Slider>() {
                slider.set_value(ctx.sound_config.bgm);
            }
        }

        if let Some(widget) = self.tree.find_widget_mut("slider_sfx") {
            if let Some(slider) = widget.as_any_mut().downcast_mut::<Slider>() {
                slider.set_value(ctx.sound_config.sfx);
            }
        }
    }

    /// Set screen size for layout.
    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.tree.set_screen_size(width, height);
        self.needs_layout = true;
    }

    /// Check if layout is needed.
    pub fn needs_layout(&self) -> bool {
        self.needs_layout
    }

    /// Clear layout flag after layout.
    pub fn clear_layout_flag(&mut self) {
        self.needs_layout = false;
    }

    /// Reset settings initialization (e.g., after cancel).
    pub fn reset_settings(&mut self) {
        self.settings_initialized = false;
    }
}

/// Parse resolution string "1920 x 1080" to (width, height).
fn parse_resolution(s: &str) -> Option<(f32, f32)> {
    let parts: Vec<&str> = s.split(" x ").collect();
    if parts.len() == 2 {
        let w: f32 = parts[0].trim().parse().ok()?;
        let h: f32 = parts[1].trim().parse().ok()?;
        Some((w, h))
    } else {
        None
    }
}

/// Parse MSAA string to level.
/// Handles both "2x" and "2x MSAA" formats.
fn parse_msaa(s: &str) -> i32 {
    match s {
        "None" => 1,
        "2x" | "2x MSAA" => 2,
        "4x" | "4x MSAA" => 4,
        "8x" | "8x MSAA" => 8,
        "16x" | "16x MSAA" => 16,
        _ => 16,
    }
}

/// Convert UI font name to actual font family name.
/// All fonts use their display name as-is.
fn ui_font_to_family(ui_name: &str) -> String {
    ui_name.to_string()
}

/// Convert font family name to UI display name.
/// All fonts use their family name as-is.
fn family_to_ui_font(family: &str) -> String {
    family.to_string()
}
