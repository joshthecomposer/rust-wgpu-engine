use std::cell::Cell;
use std::rc::Rc;

use slint::platform::software_renderer::MinimalSoftwareWindow;
use slint::PhysicalSize;

use crate::entity_manager::EntityManager;
use crate::ui::message_queue::{MessageQueue, UiMessage};

slint::include_modules!();

/// Context passed to PauseMenuView::update().
/// Contains mutable refs for state the pause menu can modify,
/// and a message queue for global messages (quit, reload world).
pub struct PauseMenuContext<'a> {
    pub paused: &'a mut bool,
    pub render_gizmos: &'a mut bool,

    pub entity_manager: &'a EntityManager,

    pub message_queue: &'a mut MessageQueue,
}

/// Manages the Slint PauseMenu component and its callbacks
pub struct PauseMenuView {
    pause_menu: PauseMenu,
    // callback state (using Cell for interior mutability in callbacks)
    close_pending: Rc<Cell<bool>>,
    gizmo_pending: Rc<Cell<bool>>,
    reload_pending: Rc<Cell<bool>>,
    save_pending: Rc<Cell<bool>>,
    quit_pending: Rc<Cell<bool>>,
}

impl PauseMenuView {
    /// Create a new PauseMenuView. Returns the view and the window it created.
    pub fn new(width: u32, height: u32) -> (Self, Rc<MinimalSoftwareWindow>) {
        let pause_menu = PauseMenu::new().unwrap();

        let window = crate::ui::slint_platform::get_last_created_window()
            .expect("Expected window to be created for PauseMenu");
        window.set_size(PhysicalSize::new(width, height));

        // create callback state cells
        let close_pending = Rc::new(Cell::new(false));
        let gizmo_pending = Rc::new(Cell::new(false));
        let reload_pending = Rc::new(Cell::new(false));
        let save_pending = Rc::new(Cell::new(false));
        let quit_pending = Rc::new(Cell::new(false));

        Self::create_callbacks(
            &pause_menu,
            &close_pending,
            &gizmo_pending,
            &reload_pending,
            &save_pending,
            &quit_pending,
        );

        let view = Self {
            pause_menu,
            close_pending,
            gizmo_pending,
            reload_pending,
            save_pending,
            quit_pending,
        };

        (view, window)
    }

    /// Create the callbacks for the pause menu
    fn create_callbacks(
        pause_menu: &PauseMenu,
        close_pending: &Rc<Cell<bool>>,
        gizmo_pending: &Rc<Cell<bool>>,
        reload_pending: &Rc<Cell<bool>>,
        save_pending: &Rc<Cell<bool>>,
        quit_pending: &Rc<Cell<bool>>,
    ) {
        {
            let close = close_pending.clone();
            pause_menu.on_close_clicked(move || {
                println!("[PauseMenu] Close clicked!");
                close.set(true);
            });
        }
        {
            let gizmo = gizmo_pending.clone();
            pause_menu.on_gizmo_clicked(move || {
                println!("[PauseMenu] Gizmo clicked!");
                gizmo.set(true);
            });
        }
        {
            let reload = reload_pending.clone();
            pause_menu.on_reload_world_clicked(move || {
                println!("[PauseMenu] Reload clicked!");
                reload.set(true);
            });
        }
        {
            let save = save_pending.clone();
            pause_menu.on_save_player_clicked(move || {
                println!("[PauseMenu] Save clicked!");
                save.set(true);
            });
        }
        {
            let quit = quit_pending.clone();
            pause_menu.on_quit_clicked(move || {
                println!("[PauseMenu] Quit clicked!");
                quit.set(true);
            });
        }
    }

    /// Update the pause menu view.
    ///
    /// This method:
    /// - Updates the pause menu visibility based on the paused state
    /// - Processes pending UI callbacks (close, gizmo toggle, reload, save, quit)
    /// - Directly modifies state via context refs for view-specific actions
    /// - Sends global messages for cross-system actions (quit, reload world)
    pub fn update(&mut self, ctx: PauseMenuContext) {
        self.pause_menu.set_show_pause_menu(*ctx.paused);

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
