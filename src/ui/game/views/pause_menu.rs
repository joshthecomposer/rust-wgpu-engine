//! Pause menu view - handles pause menu logic and callbacks.
//! Works with the GameRoot Slint component, which is owned by GameRootView.

use std::cell::Cell;
use std::rc::Rc;

use crate::entity_manager::EntityManager;
use crate::ui::message_queue::{MessageQueue, UiMessage};

use super::game_root::{GameRoot, SettingsContext, SystemContext};

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
        }
    }

    /// Update the pause menu view.
    pub fn update(&self, game_root: &GameRoot, ctx: PauseMenuContext) {
        game_root.set_show_pause_menu(*ctx.paused);

        // initialize UI from engine values on first run or after cancel
        if !self.settings_initialized.get() {
            game_root.set_gizmo_enabled(*ctx.settings.render_gizmos);
            game_root.set_show_fps(*ctx.settings.show_fps);
            game_root.set_bgm_volume(*ctx.settings.bgm_volume);
            game_root.set_sfx_volume(*ctx.settings.sfx_volume);
            game_root.set_vsync(*ctx.settings.vsync);
            game_root.set_debug_mode(*ctx.settings.debug_mode);
            self.settings_initialized.set(true);
        }

        // sync settings state from UI to engine (live preview)
        // only sync FROM UI TO engine, not the other way, to preserve user changes
        *ctx.settings.render_gizmos = game_root.get_gizmo_enabled();
        *ctx.settings.show_fps = game_root.get_show_fps();
        *ctx.settings.bgm_volume = game_root.get_bgm_volume();
        *ctx.settings.sfx_volume = game_root.get_sfx_volume();
        *ctx.settings.vsync = game_root.get_vsync();
        *ctx.settings.debug_mode = game_root.get_debug_mode();

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
