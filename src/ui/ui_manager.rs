//! UI Manager - Abstracts Ultralight HTML/CSS/JS UI system
//!
//! This module provides a high-level interface for the game's UI system,
//! wrapping the UltralightManager and handling UI state management.

use winit::event::WindowEvent;

use crate::ultralight::manager::UltralightManager;
use crate::ultralight::types::ViewType;

use super::events::{parse_js_event, UiEvent};

/// High-level UI manager that wraps UltralightManager and handles UI state.
pub struct UiManager {
    /// Ultralight HTML/CSS/JS UI manager (optional, may fail to initialize)
    ultralight: Option<UltralightManager>,
    /// Track if editor was visible last frame to avoid spamming show/hide
    pub editor_was_visible: bool,
    /// Track if pause menu was visible last frame to avoid spamming show/hide
    pub pause_menu_was_visible: bool,
}

impl UiManager {
    /// Create a new UiManager with initialized Ultralight views.
    pub fn new(fb_width: u32, fb_height: u32) -> Self {
        let ultralight = match UltralightManager::new(fb_width, fb_height) {
            Ok(mut manager) => {
                println!("[UiManager] Ultralight initialized successfully");

                // Create the Editor view (replaces ImGui editor)
                if let Err(e) = manager.create_view(ViewType::Editor, fb_width, fb_height) {
                    eprintln!("[UiManager] Failed to create Editor view: {:?}", e);
                } else {
                    // Load editor HTML
                    if let Err(e) = manager.load_url(ViewType::Editor, "file:///resources/ui/editor.html") {
                        eprintln!("[UiManager] Failed to load Editor HTML: {:?}", e);
                    }
                }

                // Create the PauseMenu view (replaces old game_ui pause menu)
                if let Err(e) = manager.create_view(ViewType::PauseMenu, fb_width, fb_height) {
                    eprintln!("[UiManager] Failed to create PauseMenu view: {:?}", e);
                } else {
                    // Load pause menu HTML
                    if let Err(e) = manager.load_url(ViewType::PauseMenu, "file:///resources/ui/pause_menu.html") {
                        eprintln!("[UiManager] Failed to load PauseMenu HTML: {:?}", e);
                    }
                }

                Some(manager)
            }
            Err(e) => {
                eprintln!("[UiManager] Failed to initialize Ultralight: {:?}", e);
                None
            }
        };

        Self {
            ultralight,
            editor_was_visible: false,
            pause_menu_was_visible: false,
        }
    }

    /// Check if Ultralight is initialized.
    pub fn is_initialized(&self) -> bool {
        self.ultralight.is_some()
    }

    /// Get mutable reference to the underlying UltralightManager.
    pub fn ultralight_mut(&mut self) -> Option<&mut UltralightManager> {
        self.ultralight.as_mut()
    }

    /// Get reference to the underlying UltralightManager.
    pub fn ultralight(&self) -> Option<&UltralightManager> {
        self.ultralight.as_ref()
    }

    /// Handle a window event and return (wants_mouse, wants_keyboard).
    pub fn handle_event(&mut self, event: &WindowEvent) -> (bool, bool) {
        if let Some(ref mut ultralight) = self.ultralight {
            let wants_mouse = ultralight.handle_event(event);
            let wants_keyboard = ultralight.want_capture_keyboard;
            (wants_mouse, wants_keyboard)
        } else {
            (false, false)
        }
    }

    /// Handle window resize.
    pub fn handle_resize(&mut self, width: u32, height: u32) {
        if let Some(ref mut ultralight) = self.ultralight {
            ultralight.handle_window_resize(width, height);
        }
    }

    /// Show a view.
    pub fn show_view(&mut self, view_type: ViewType) {
        if let Some(ref mut ultralight) = self.ultralight {
            let _ = ultralight.show_view(view_type);
        }
    }

    /// Hide a view.
    pub fn hide_view(&mut self, view_type: ViewType) {
        if let Some(ref mut ultralight) = self.ultralight {
            let _ = ultralight.hide_view(view_type);
        }
    }

    /// Execute JavaScript in a view.
    pub fn execute_js(&mut self, view_type: ViewType, script: &str) -> Option<String> {
        if let Some(ref mut ultralight) = self.ultralight {
            ultralight.execute_js(view_type, script).ok()
        } else {
            None
        }
    }

    /// Update the UI each frame.
    pub fn update(&mut self, dt: f32) {
        if let Some(ref mut ultralight) = self.ultralight {
            use crate::ultralight::types::ExposedGameState;
            let game_state = ExposedGameState::default();
            ultralight.update(&game_state, dt);
        }
    }

    /// Drain all pending JS events as raw JSON strings (deprecated, use poll_events).
    pub fn drain_js_events(&mut self) -> Vec<String> {
        if let Some(ref mut ultralight) = self.ultralight {
            ultralight.drain_js_events()
        } else {
            Vec::new()
        }
    }

    /// Poll for UI events, returning structured UiEvent objects.
    /// This parses raw JS events into typed events that Game can handle.
    pub fn poll_events(&mut self) -> Vec<UiEvent> {
        self.drain_js_events()
            .iter()
            .filter_map(|json| parse_js_event(json))
            .collect()
    }

    /// Render all visible UI views.
    pub fn render(&mut self, fb_width: u32, fb_height: u32) {
        if let Some(ref mut ultralight) = self.ultralight {
            ultralight.render(fb_width, fb_height);
        }
    }

    /// Update editor dropdowns with entity types, factions, and emitter types.
    pub fn update_editor_dropdowns(
        &mut self,
        entity_types: &[String],
        factions: &[String],
        emitter_types: &[String],
    ) {
        if let Some(ref mut ultralight) = self.ultralight {
            ultralight.update_editor_dropdowns(entity_types, factions, emitter_types);
        }
    }

    /// Update the editor with current player state.
    pub fn update_editor_state(
        &mut self,
        player_pos: [f32; 3],
        player_state: &str,
        attack_state: &str,
        animation: &str,
    ) {
        if let Some(ref mut ultralight) = self.ultralight {
            ultralight.update_editor_state(player_pos, player_state, attack_state, animation);
        }
    }

    /// Update the emitter position in the editor.
    pub fn update_emitter_position(&mut self, pos: [f32; 3]) {
        if let Some(ref mut ultralight) = self.ultralight {
            ultralight.update_emitter_position(pos);
        }
    }

    /// Show or hide editor based on state change.
    /// Returns true if editor is now visible.
    pub fn update_editor_visibility(&mut self, should_show: bool) -> bool {
        if should_show != self.editor_was_visible {
            if should_show {
                self.show_view(ViewType::Editor);
            } else {
                self.hide_view(ViewType::Editor);
            }
            self.editor_was_visible = should_show;
        }
        should_show
    }

    /// Show or hide pause menu based on state change.
    /// Also updates gizmo status in the UI.
    pub fn update_pause_menu_visibility(&mut self, paused: bool, render_gizmos: bool) {
        if paused != self.pause_menu_was_visible {
            if paused {
                self.show_view(ViewType::PauseMenu);
                // Update gizmo status when showing pause menu
                let gizmo_status = if render_gizmos { "true" } else { "false" };
                self.execute_js(ViewType::PauseMenu, &format!("updateGizmoStatus({})", gizmo_status));
            } else {
                self.hide_view(ViewType::PauseMenu);
            }
            self.pause_menu_was_visible = paused;
        }
    }

    /// Notify editor that an entity was placed.
    pub fn notify_entity_placed(&mut self) {
        self.execute_js(ViewType::Editor, "window.editorAPI.onEntityPlaced()");
    }
}

