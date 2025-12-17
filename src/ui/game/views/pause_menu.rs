//! Pause menu view - handles pause menu logic and callbacks.
//! Works with the GameRoot Slint component, which is owned by GameRootView.

use std::cell::Cell;
use std::rc::Rc;

use crate::entity_manager::EntityManager;
use crate::ui::message_queue::{MessageQueue, UiMessage};

use super::game_root::GameRoot;

/// Context passed to PauseMenuView::update().
pub struct PauseMenuContext<'a> {
    pub paused: &'a mut bool,
    pub render_gizmos: &'a mut bool,
    pub entity_manager: &'a EntityManager,
    pub message_queue: &'a mut MessageQueue,
}

/// Manages the pause menu portion of the GameRoot component.
pub struct PauseMenuView {
    close_pending: Rc<Cell<bool>>,
    gizmo_pending: Rc<Cell<bool>>,
    reload_pending: Rc<Cell<bool>>,
    save_pending: Rc<Cell<bool>>,
    quit_pending: Rc<Cell<bool>>,
}

impl PauseMenuView {
    /// Create a new PauseMenuView and wire up callbacks to the GameRoot component.
    pub fn new(game_root: &GameRoot) -> Self {
        let close_pending = Rc::new(Cell::new(false));
        let gizmo_pending = Rc::new(Cell::new(false));
        let reload_pending = Rc::new(Cell::new(false));
        let save_pending = Rc::new(Cell::new(false));
        let quit_pending = Rc::new(Cell::new(false));

        // wire up callbacks
        {
            let close = close_pending.clone();
            game_root.on_close_clicked(move || close.set(true));
        }
        {
            let gizmo = gizmo_pending.clone();
            game_root.on_gizmo_clicked(move || gizmo.set(true));
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

        Self {
            close_pending,
            gizmo_pending,
            reload_pending,
            save_pending,
            quit_pending,
        }
    }

    /// Update the pause menu view.
    pub fn update(&self, game_root: &GameRoot, ctx: PauseMenuContext) {
        game_root.set_show_pause_menu(*ctx.paused);

        self.handle_unpause(ctx.paused);
        self.handle_toggle_gizmos(ctx.render_gizmos);
        self.handle_reload_world(ctx.message_queue);
        self.handle_save_player_data(ctx.entity_manager);
        self.handle_quit(ctx.message_queue);
    }
    /// Handle unpause action by directly modifying the paused state.
    /// This is a view-specific action, so we modify state directly via context ref.
    fn handle_unpause(&self, paused: &mut bool) {
        if self.close_pending.replace(false) {
            *paused = false;
        }
    }

    /// Handle gizmo rendering toggle by directly modifying the render_gizmos state.
    /// This is a view-specific action, so we modify state directly via context ref.
    fn handle_toggle_gizmos(&self, render_gizmos: &mut bool) {
        if self.gizmo_pending.replace(false) {
            *render_gizmos = !*render_gizmos;
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
