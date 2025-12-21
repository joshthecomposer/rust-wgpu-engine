//! Pause menu view - handles pause menu logic and callbacks.
//! Works with the GameRoot Slint component, which is owned by GameRootView.

use std::cell::Cell;
use std::rc::Rc;

use crate::entity_manager::EntityManager;
use crate::toast;
use crate::ui::message_queue::{MessageQueue, UiMessage};

use super::game_root::{GameRoot, SettingsContext, SystemContext};

/// Convert MSAA level (i32) to display string.
/// 1 -> "None", 2 -> "2x MSAA", 4 -> "4x MSAA", 8 -> "8x MSAA", 16 -> "16x MSAA"
fn msaa_level_to_string(level: i32) -> String {
    match level {
        1 => "None".to_string(),
        2 => "2x MSAA".to_string(),
        4 => "4x MSAA".to_string(),
        8 => "8x MSAA".to_string(),
        16 => "16x MSAA".to_string(),
        _ => {
            eprintln!(
                "Warning: Invalid MSAA level {}, defaulting to 16x MSAA",
                level
            );
            "16x MSAA".to_string()
        }
    }
}

/// Convert MSAA display string to level (i32).
/// "None" -> 1, "2x MSAA" -> 2, "4x MSAA" -> 4, "8x MSAA" -> 8, "16x MSAA" -> 16
fn msaa_string_to_level(s: &str) -> i32 {
    match s {
        "None" => 1,
        "2x MSAA" => 2,
        "4x MSAA" => 4,
        "8x MSAA" => 8,
        "16x MSAA" => 16,
        _ => {
            eprintln!("Warning: Invalid MSAA string '{}', defaulting to 16", s);
            16
        }
    }
}

/// Convert UI font name to actual font family name.
/// "JetBrains Mono" -> "JetBrains Mono Medium" (because we use the Medium weight TTF)
/// All other fonts use their display name as-is.
fn ui_font_to_family(ui_name: &str) -> String {
    match ui_name {
        "JetBrains Mono" => "JetBrains Mono Medium".to_string(),
        other => other.to_string(),
    }
}

/// Convert font family name to UI display name.
/// "JetBrains Mono Medium" -> "JetBrains Mono"
/// All other fonts use their family name as-is.
fn family_to_ui_font(family: &str) -> String {
    match family {
        "JetBrains Mono Medium" => "JetBrains Mono".to_string(),
        other => other.to_string(),
    }
}

/// Context passed to PauseMenuView::update().
/// Uses nested contexts to logically group settings and system resources.
pub struct PauseMenuContext<'a> {
    pub paused: &'a mut bool,
    pub settings: SettingsContext<'a>,
    pub system: SystemContext<'a>,
}

/// Manages the pause menu portion of the GameRoot component.
pub struct PauseMenuView {
    close_pending: Rc<Cell<bool>>,
    reload_pending: Rc<Cell<bool>>,
    save_pending: Rc<Cell<bool>>,
    quit_pending: Rc<Cell<bool>>,
    apply_settings_pending: Rc<Cell<bool>>,
    cancel_settings_pending: Rc<Cell<bool>>,
    settings_initialized: Rc<Cell<bool>>, // track if we've initialized UI from engine
    last_msaa_level: Rc<Cell<i32>>,       // track last MSAA level for debug logging
}

impl PauseMenuView {
    /// Create a new PauseMenuView and wire up callbacks to the GameRoot component.
    pub fn new(game_root: &GameRoot) -> Self {
        let close_pending = Rc::new(Cell::new(false));
        let reload_pending = Rc::new(Cell::new(false));
        let save_pending = Rc::new(Cell::new(false));
        let quit_pending = Rc::new(Cell::new(false));
        let apply_settings_pending = Rc::new(Cell::new(false));
        let cancel_settings_pending = Rc::new(Cell::new(false));

        // wire up callbacks
        {
            let close = close_pending.clone();
            game_root.on_close_clicked(move || close.set(true));
        }
        {
            let reload = reload_pending.clone();
            game_root.on_reload_world_clicked(move || reload.set(true));
        }
        {
            let save = save_pending.clone();
            game_root.on_save_player_clicked(move || save.set(true));
        }
        {
            let quit = quit_pending.clone();
            game_root.on_quit_clicked(move || quit.set(true));
        }
        {
            let apply = apply_settings_pending.clone();
            game_root.on_apply_settings_clicked(move || apply.set(true));
        }
        {
            let cancel = cancel_settings_pending.clone();
            game_root.on_cancel_settings_clicked(move || cancel.set(true));
        }

        Self {
            close_pending,
            reload_pending,
            save_pending,
            quit_pending,
            apply_settings_pending,
            cancel_settings_pending,
            settings_initialized: Rc::new(Cell::new(false)),
            last_msaa_level: Rc::new(Cell::new(-1)), // -1 = uninitialized
        }
    }

    /// Update the pause menu view.
    pub fn update(&self, game_root: &GameRoot, ctx: PauseMenuContext) {
        game_root.set_show_pause_menu(*ctx.paused);

        // initialize UI from engine values on first run or after cancel
        if !self.settings_initialized.get() {
            // format resolution as "width x height"
            let resolution_str = format!(
                "{} x {}",
                ctx.settings.game_config.win_width, ctx.settings.game_config.win_height
            );
            game_root.set_resolution(resolution_str.into());
            game_root.set_window_mode(ctx.settings.game_config.window_mode.clone().into());
            game_root.set_gizmo_enabled(*ctx.settings.render_gizmos);
            game_root.set_show_fps(ctx.settings.game_config.fps_counter);
            game_root.set_bgm_volume(ctx.settings.sound_config.bgm);
            game_root.set_sfx_volume(ctx.settings.sound_config.sfx);
            game_root.set_vsync(ctx.settings.game_config.vsync);
            game_root.set_debug_mode(ctx.settings.game_config.debug_mode);
            game_root.set_msaa(msaa_level_to_string(ctx.settings.game_config.msaa_level).into());
            // set font family - UI shows simplified name, actual-font-family uses full name for rendering
            game_root
                .set_font_family(family_to_ui_font(&ctx.settings.game_config.font_family).into());
            game_root.set_actual_font_family(ctx.settings.game_config.font_family.clone().into());
            self.settings_initialized.set(true);
        }

        // sync settings state from UI to engine (live preview)
        // only sync FROM UI TO engine, not the other way, to preserve user changes

        // parse resolution string "width x height" back to u32 values
        let resolution_str = game_root.get_resolution().to_string();
        if let Some((width_str, height_str)) = resolution_str.split_once(" x ") {
            if let (Ok(width), Ok(height)) = (width_str.trim().parse(), height_str.trim().parse()) {
                ctx.settings.game_config.win_width = width;
                ctx.settings.game_config.win_height = height;
            }
        }

        ctx.settings.game_config.window_mode = game_root.get_window_mode().to_string();
        *ctx.settings.render_gizmos = game_root.get_gizmo_enabled();
        ctx.settings.game_config.fps_counter = game_root.get_show_fps();
        ctx.settings.sound_config.bgm = game_root.get_bgm_volume();
        ctx.settings.sound_config.sfx = game_root.get_sfx_volume();
        ctx.settings.game_config.vsync = game_root.get_vsync();
        ctx.settings.game_config.debug_mode = game_root.get_debug_mode();

        let msaa_str = game_root.get_msaa().to_string();
        let msaa_level = msaa_string_to_level(&msaa_str);

        ctx.settings.game_config.msaa_level = msaa_level;

        let ui_font_name = game_root.get_font_family().to_string();
        let font_family = ui_font_to_family(&ui_font_name);

        // debug logging to verify font family names
        if ctx.settings.game_config.font_family != font_family {
            println!(
                "Font changed: '{}' -> '{}'",
                ctx.settings.game_config.font_family, font_family
            );
        }

        ctx.settings.game_config.font_family = font_family.clone();
        // also update the actual font family for rendering
        game_root.set_actual_font_family(font_family.into());

        self.handle_unpause(ctx.paused);
        self.handle_reload_world(ctx.system.message_queue);
        self.handle_save_player_data(ctx.system.entity_manager);
        self.handle_quit(ctx.system.message_queue);
        self.handle_apply_settings(ctx.system.message_queue);
        self.handle_cancel_settings(ctx.system.message_queue);
    }
    /// Handle unpause action by directly modifying the paused state.
    /// This is a view-specific action, so we modify state directly via context ref.
    fn handle_unpause(&self, paused: &mut bool) {
        if self.close_pending.replace(false) {
            *paused = false;
        }
    }

    /// Handle apply settings by sending a global message to save config to disk.
    fn handle_apply_settings(&self, message_queue: &mut MessageQueue) {
        if self.apply_settings_pending.replace(false) {
            message_queue.send(UiMessage::ApplySettings);
        }
    }

    /// Handle cancel settings by sending a global message to reload config from disk.
    fn handle_cancel_settings(&self, message_queue: &mut MessageQueue) {
        if self.cancel_settings_pending.replace(false) {
            message_queue.send(UiMessage::CancelSettings);
            // reset flag so UI will be reloaded from engine on next update
            self.settings_initialized.set(false);
        }
    }

    /// Handle world reload by sending a global message to the message queue.
    /// This is a global action that requires coordination across systems.
    fn handle_reload_world(&self, message_queue: &mut MessageQueue) {
        if self.reload_pending.replace(false) {
            message_queue.send(UiMessage::ReloadWorldData);
        }
    }

    /// Handle player data save by directly calling the entity manager.
    /// This is a view-specific action that doesn't require global coordination.
    fn handle_save_player_data(&self, entity_manager: &EntityManager) {
        if self.save_pending.replace(false) {
            entity_manager.serialize_entity_data("config/player_data.json");
            toast!(
                Success,
                "Player Data Saved",
                "Your player data has been saved successfully."
            );
        }
    }

    /// Handle quit action by sending a global message to the message queue.
    /// This is a global action that requires coordination with the event loop.
    fn handle_quit(&self, message_queue: &mut MessageQueue) {
        if self.quit_pending.replace(false) {
            message_queue.send(UiMessage::WindowShouldClose);
        }
    }
}
