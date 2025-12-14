use std::cell::Cell;
use std::rc::Rc;

use slint::platform::software_renderer::MinimalSoftwareWindow;
use slint::PhysicalSize;

use crate::entity_manager::EntityManager;
use crate::ui::message_queue::{MessageQueue, UiMessage};

slint::include_modules!();

/// Context passed to PauseMenuView::update() containing mutable references to game state.
/// This allows the pause menu to modify game state directly (e.g., toggle pause, gizmos)
/// and send messages to other systems via the message queue.
pub struct PauseMenuContext<'a> {
    pub paused: &'a mut bool,
    pub render_gizmos: &'a mut bool,
    pub message_queue: &'a mut MessageQueue,
    pub entity_manager: &'a EntityManager,
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

    /// Update the pause menu. Handles visibility and processes callbacks.
    pub fn update(&mut self, ctx: PauseMenuContext) {
        self.pause_menu.set_show_pause_menu(*ctx.paused);

        // handle close menu
        if self.close_pending.replace(false) {
            *ctx.paused = false;
        }

        // handle toggle gizmo rendering
        if self.gizmo_pending.replace(false) {
            *ctx.render_gizmos = !*ctx.render_gizmos;
        }

        // handle reload world data
        if self.reload_pending.replace(false) {
            ctx.message_queue.send(UiMessage::ReloadWorldData);
        }

        // handle save player data
        if self.save_pending.replace(false) {
            ctx.entity_manager
                .serialize_entity_data("config/player_data.json");
        }

        // handle quit game
        if self.quit_pending.replace(false) {
            ctx.message_queue.send(UiMessage::WindowShouldClose);
        }
    }
}
