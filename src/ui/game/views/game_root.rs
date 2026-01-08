//! Unified game UI view that manages both the pause menu and player HUD.
//! Uses a single Slint component (GameRoot) for all in-game UI, but delegates
//! to individual view structs for separation of concerns.

use std::rc::Rc;

use slint::platform::software_renderer::MinimalSoftwareWindow;
use slint::PhysicalSize;

use crate::entity_manager::EntityManager;
use crate::ui::image_cache::UiImageCache;
use crate::ui::message_queue::MessageQueue;

use super::ability_bar::AbilityBarData;
use super::pause_menu::{PauseMenuContext, PauseMenuView};
use super::player_hud::PlayerHudView;
use super::toast::ToastView;

slint::include_modules!();

/// Update interval for pickup indicator check in seconds.
/// 10 Hz is plenty responsive for showing/hiding the pickup prompt.
const PICKUP_CHECK_INTERVAL: f64 = 0.1;

/// Context for settings-related state (debug, audio, graphics, etc.).
/// Groups all user-configurable settings to avoid bloating parent contexts.
pub struct SettingsContext<'a> {
    pub render_gizmos: &'a mut bool, // kept separate since it's in renderer, not config
    pub game_config: &'a mut crate::config::game_config::GameConfig,
    pub sound_config: &'a mut crate::config::sound_config::SoundConfig,
}

/// Context for system-level resources needed for system actions.
/// Groups resources needed for save/load, quit, reload, etc.
pub struct SystemContext<'a> {
    pub entity_manager: &'a EntityManager,
    pub message_queue: &'a mut MessageQueue,
}

/// Context passed to GameRootView::update().
/// Uses nested contexts to logically group related fields.
pub struct GameRootContext<'a> {
    pub paused: &'a mut bool,
    pub settings: SettingsContext<'a>,
    pub system: SystemContext<'a>,
    pub image_cache: &'a mut UiImageCache,
    pub elapsed_time: f64,
}

/// Manages the unified GameRoot Slint component and delegates to child views.
pub struct GameRootView {
    game_root: GameRoot,
    pause_menu_view: PauseMenuView,
    player_hud_view: PlayerHudView,
    toast_view: ToastView,
    // throttling for pickup indicator check
    last_pickup_check_time: f64,
    cached_show_pickup: bool,
    cached_ability_data: Option<AbilityBarData>,
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
        let toast_view = ToastView::new(&game_root);

        let view = Self {
            game_root,
            pause_menu_view,
            player_hud_view,
            toast_view,
            last_pickup_check_time: -999.0, // force first check
            cached_show_pickup: false,
            cached_ability_data: None,
        };

        (view, window)
    }

    /// Update the game root view by delegating to child views.
    pub fn update(&mut self, ctx: GameRootContext) {
        let paused = *ctx.paused;
        let entity_manager = ctx.system.entity_manager;
        let elapsed_time = ctx.elapsed_time;

        // drain pending toasts from global queue and add them
        let pending_toasts = crate::ui::toast::drain_pending_toasts();
        for toast in pending_toasts {
            self.toast_view.add_toast(
                toast.toast_type,
                toast.title,
                toast.message,
                toast.duration,
                elapsed_time,
            );
        }

        // throttle pickup indicator check to 10 Hz (avoid expensive entity iteration every frame)
        let should_check = elapsed_time - self.last_pickup_check_time >= PICKUP_CHECK_INTERVAL;

        if should_check {
            let show_pickup_prompt = !paused && entity_manager.has_nearby_weapon();
            self.cached_show_pickup = show_pickup_prompt;
            self.last_pickup_check_time = elapsed_time;
        }

        self.game_root
            .set_show_pickup_prompt(self.cached_show_pickup);

        // delegate to pause menu view
        let pause_ctx = PauseMenuContext {
            paused: ctx.paused,
            settings: ctx.settings,
            system: ctx.system,
        };
        self.pause_menu_view.update(&self.game_root, pause_ctx);

        self.game_root.set_show_player_hud(false);

        // update ability bar slot data on game_root for tooltip hover detection
        // (the visual rendering is done separately by AbilityBarRenderer, but GameRoot
        // needs the data for its TouchArea hover detection to show tooltips)
        let ability_data = AbilityBarData::from_entity_manager(entity_manager);

        // change detection: for tooltips, we only care about structural changes (name, description, visibility)
        // we don't need to update Slint properties for cooldown progress every frame in GameRoot!
        let needs_ability_sync = match &self.cached_ability_data {
            Some(cached) => {
                cached.visible != ability_data.visible
                    || cached.m1.ability_id != ability_data.m1.ability_id
                    || cached.m2.ability_id != ability_data.m2.ability_id
                    || cached.q.ability_id != ability_data.q.ability_id
                    || cached.e.ability_id != ability_data.e.ability_id
                    || cached.shift.ability_id != ability_data.shift.ability_id
                    || cached.r.ability_id != ability_data.r.ability_id
                    || cached.m1.ability_name != ability_data.m1.ability_name
                    || cached.m2.ability_name != ability_data.m2.ability_name
                    || cached.q.ability_name != ability_data.q.ability_name
                    || cached.e.ability_name != ability_data.e.ability_name
                    || cached.shift.ability_name != ability_data.shift.ability_name
                    || cached.r.ability_name != ability_data.r.ability_name
            }
            None => true,
        };

        self.toast_view.update(&self.game_root, elapsed_time);
    }

    /// Set the current FPS for the FPS counter.
    pub fn set_fps(&self, fps: i32) {
        self.game_root.set_current_fps(fps);
    }
}
