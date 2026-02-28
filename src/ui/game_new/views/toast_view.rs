//! ToastView - manages the toast notification UI in the custom GPU UI system.
//!
//! Replaces the Slint-based toast implementation.

use std::collections::HashMap;

use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::parser::load_view_or_fallback;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::tree::UiTree;
use crate::ui::game_new::widgets::ToastContainer;
use crate::ui::toast::ToastType;

/// ToastView manages the toast notification portion of the game HUD.
///
/// Position is at bottom-right of the screen.
pub struct ToastView {
    tree: UiTree,
    pub needs_layout: bool,
    screen_width: f32,
    screen_height: f32,
    elapsed_time: f64,
    /// Cache of loaded icon textures (type -> texture_id)
    icon_cache: HashMap<ToastType, u32>,
}

impl ToastView {
    /// Create a new ToastView.
    pub fn new(font_system: &mut FontSystem) -> Self {
        let mut tree = load_view_or_fallback("resources/ui/toast_container.ron");

        // TODO: get the actual screen size from the window
        let screen_width = 1920.0;
        let screen_height = 1080.0;
        tree.set_screen_size(screen_width, screen_height);
        tree.layout(font_system);

        Self {
            tree,
            needs_layout: false,
            screen_width,
            screen_height,
            elapsed_time: 0.0,
            icon_cache: HashMap::new(),
        }
    }

    /// Set the screen size for positioning.
    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        if self.screen_width != width || self.screen_height != height {
            self.screen_width = width;
            self.screen_height = height;
            self.needs_layout = true;
        }
    }

    /// Update elapsed time for animation tracking (deprecated - use set_elapsed_time instead).
    pub fn update_time(&mut self, delta: f64) {
        self.elapsed_time += delta;
    }

    /// Set the absolute elapsed time for animation tracking.
    /// This should be called with the game's elapsed time, not a delta.
    pub fn set_elapsed_time(&mut self, elapsed_time: f64) {
        self.elapsed_time = elapsed_time;
    }

    /// Add a new toast notification.
    pub fn add_toast(
        &mut self,
        toast_type: ToastType,
        title: String,
        message: String,
        duration: Option<f64>,
    ) {
        println!(
            "[ToastView::add_toast] Adding toast: {:?} - {}",
            toast_type, title
        );
        // get icon texture for this toast type (if cached)
        let icon_texture = self.icon_cache.get(&toast_type).copied();

        // find the ToastContainer widget and add the toast
        if let Some(w) = self.tree.find_widget_mut("toast_container") {
            if let Some(container) = w.as_any_mut().downcast_mut::<ToastContainer>() {
                container.add_toast(
                    toast_type,
                    title.clone(),
                    message,
                    duration,
                    self.elapsed_time,
                    icon_texture,
                );
                self.needs_layout = true;
                println!("[ToastView::add_toast] Set needs_layout = true");
            } else {
                println!("[ToastView::add_toast] ERROR: Could not downcast to ToastContainer!");
            }
        } else {
            println!("[ToastView::add_toast] ERROR: Could not find toast_container widget!");
        }
    }

    /// Layout the toast container.
    pub fn layout(&mut self, font_system: &mut FontSystem) {
        if self.needs_layout {
            println!("[ToastView::layout] Calling tree.layout()");
            self.tree
                .set_screen_size(self.screen_width, self.screen_height);
            // CRITICAL: force_layout() sets tree.needs_layout = true
            // Without this, tree.layout() returns early if tree.needs_layout is false
            self.tree.force_layout();
            self.tree.layout(font_system);
            self.needs_layout = false;
            println!("[ToastView::layout] Layout complete, needs_layout = false");
        }
    }

    /// Update the toast container (handle animation states, remove expired toasts).
    pub fn update(&mut self, ctx: &mut UiContext) -> bool {
        // update toast states based on elapsed time
        if let Some(w) = self.tree.find_widget_mut("toast_container") {
            if let Some(container) = w.as_any_mut().downcast_mut::<ToastContainer>() {
                container.update_states(self.elapsed_time);
            }
        }

        // regular widget update (handles input)
        let consumed = self.tree.update(ctx);

        // check if layout needs update (e.g., toast was removed)
        if let Some(w) = self.tree.find_widget_mut("toast_container") {
            if let Some(container) = w.as_any_mut().downcast_mut::<ToastContainer>() {
                if container.needs_layout_update {
                    self.needs_layout = true;
                    container.needs_layout_update = false;
                }
            }
        }

        consumed
    }

    /// Render the toast container.
    pub fn render(&self, renderer: &mut UiRenderer) {
        self.tree.render(renderer);
    }

    /// Load an icon texture for a toast type.
    /// This should be called during initialization to preload icons.
    pub fn load_icon(&mut self, toast_type: ToastType, texture_id: u32) {
        self.icon_cache.insert(toast_type, texture_id);
    }
}
