//! Unified game UI view that manages both the pause menu and player HUD.
//! Uses a single Slint component (GameRoot) for all in-game UI, but delegates
//! to individual view structs for separation of concerns.

use std::rc::Rc;

use slint::platform::software_renderer::MinimalSoftwareWindow;
use slint::PhysicalSize;

use crate::entity_manager::EntityManager;
use crate::ui::message_queue::MessageQueue;

use super::pause_menu::{PauseMenuContext, PauseMenuView};
use super::player_hud::{PlayerHudContext, PlayerHudView};

slint::include_modules!();

/// Context passed to GameRootView::update().
pub struct GameRootContext<'a> {
    pub paused: &'a mut bool,
    pub render_gizmos: &'a mut bool,
    pub entity_manager: &'a EntityManager,
    pub message_queue: &'a mut MessageQueue,
}

/// Manages the unified GameRoot Slint component and delegates to child views.
pub struct GameRootView {
    game_root: GameRoot,
    pause_menu_view: PauseMenuView,
    player_hud_view: PlayerHudView,
}

impl GameRootView {
    /// Create a new GameRootView. Returns the view and the window it created.
    pub fn new(width: u32, height: u32, scale_factor: f32) -> (Self, Rc<MinimalSoftwareWindow>) {
        let game_root = GameRoot::new().unwrap();

        let window = crate::ui::slint_platform::get_last_created_window()
            .expect("Expected window to be created for GameRoot");

        // IMPORTANT: set scale factor FIRST, before set_size, so the logical size is computed correctly
        window.dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged { scale_factor });

        // now set the physical size - this will compute logical size using the scale factor
        window.set_size(PhysicalSize::new(width, height));

        // create child views that wire up their callbacks to game_root
        let pause_menu_view = PauseMenuView::new(&game_root);
        let player_hud_view = PlayerHudView::new();

        let view = Self {
            game_root,
            pause_menu_view,
            player_hud_view,
        };

        (view, window)
    }

    /// Update the game root view by delegating to child views.
    pub fn update(&mut self, ctx: GameRootContext) {
        let paused = *ctx.paused;

        // delegate to pause menu view
        let pause_ctx = PauseMenuContext {
            paused: ctx.paused,
            render_gizmos: ctx.render_gizmos,
            entity_manager: ctx.entity_manager,
            message_queue: ctx.message_queue,
        };
        self.pause_menu_view.update(&self.game_root, pause_ctx);

        // delegate to player HUD view
        let hud_ctx = PlayerHudContext {
            entity_manager: ctx.entity_manager,
            paused,
        };
        self.player_hud_view.update(&self.game_root, hud_ctx);
    }
}
