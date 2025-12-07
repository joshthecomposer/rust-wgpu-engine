#![allow(dead_code)]

use ul_next::event::{KeyEvent, MouseEvent, ScrollEvent};
use ul_next::view::View;

use super::types::{ViewConfig, ViewType, UltralightError};

/// Wrapper around an Ultralight view with game engine integration.
pub struct UltralightView {
    /// The type of this view
    pub view_type: ViewType,
    /// Configuration used to create this view
    pub config: ViewConfig,
    /// Whether this view is currently visible
    pub visible: bool,
    /// Whether this view currently has focus
    pub focused: bool,
    /// OpenGL texture ID for GPU-rendered views (0 if CPU rendered)
    pub gl_texture_id: u32,
    /// Screen rectangle (x, y, width, height)
    pub screen_rect: (i32, i32, u32, u32),
    /// Z-index for rendering order
    pub z_index: i32,
    /// Whether the view wants to capture mouse input
    pub want_capture_mouse: bool,
    /// Whether the view wants to capture keyboard input
    pub want_capture_keyboard: bool,
    /// The actual Ultralight view
    ul_view: View,
}

impl UltralightView {
    /// Create a new view wrapping an Ultralight View.
    ///
    /// This is called by UltralightManager after creating the ul_next::View.
    pub fn new(
        view_type: ViewType,
        width: u32,
        height: u32,
        ul_view: View,
    ) -> Result<Self, UltralightError> {
        let mut config = ViewConfig::for_view_type(view_type, width, height);
        config.gpu_accelerated = false; // Currently only CPU rendering

        Ok(Self {
            view_type,
            config: config.clone(),
            visible: config.visible,
            focused: false,
            gl_texture_id: 0,
            screen_rect: (config.position.0, config.position.1, width, height),
            z_index: config.z_index,
            want_capture_mouse: false,
            want_capture_keyboard: false,
            ul_view,
        })
    }

    /// Load HTML content from a URL.
    pub fn load_url(&mut self, url: &str) -> Result<(), UltralightError> {
        println!("[Ultralight] Loading URL: {}", url);
        self.ul_view.load_url(url)
            .map_err(|e| UltralightError::LoadFailed(format!("Failed to load URL: {:?}", e)))
    }

    /// Load HTML content from a string.
    pub fn load_html(&mut self, html: &str) -> Result<(), UltralightError> {
        println!("[Ultralight] Loading HTML ({} bytes)", html.len());
        self.ul_view.load_html(html)
            .map_err(|e| UltralightError::LoadFailed(format!("Failed to load HTML: {:?}", e)))
    }

    /// Execute JavaScript in this view and return the result as a string.
    ///
    /// If the JS returns a string, it's returned directly.
    /// For other types, they are converted to JSON.
    pub fn execute_js(&mut self, script: &str) -> Result<String, UltralightError> {
        // Lock the JS context and evaluate the script
        let ctx = self.ul_view.lock_js_context();

        // Evaluate and extract the result string while ctx is still alive
        let result: Result<String, UltralightError> = match ctx.evaluate_script(script) {
            Ok(value) => {
                // If it's already a string, use as_string to avoid double-encoding
                // Otherwise use to_json_string for objects/arrays
                let result_string = if value.is_string() {
                    if let Ok(js_string) = value.as_string() {
                        js_string.to_string()
                    } else {
                        String::new()
                    }
                } else if let Ok(js_string) = value.to_json_string() {
                    js_string.to_string()
                } else {
                    String::new()
                };
                Ok(result_string)
            }
            Err(_) => Err(UltralightError::JsError("JS execution failed".to_string()))
        };

        // ctx is dropped here, result_string is already an owned String
        result
    }

    /// Get the surface pixels for CPU-rendered views.
    ///
    /// Returns None if the view is GPU-accelerated or has no surface.
    pub fn get_surface_pixels(&mut self) -> Option<Vec<u8>> {
        // Get the surface from the view (only available for non-accelerated views)
        if let Some(mut surface) = self.ul_view.surface() {
            // Lock the pixels and copy them
            if let Some(pixels) = surface.lock_pixels() {
                return Some(pixels.to_vec());
            }
        }
        None
    }

    /// Resize the view.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.screen_rect.2 = width;
        self.screen_rect.3 = height;
        // TODO: Resize actual Ultralight view
    }

    /// Set the screen position of this view.
    pub fn set_position(&mut self, x: i32, y: i32) {
        self.config.position = (x, y);
        self.screen_rect.0 = x;
        self.screen_rect.1 = y;
    }

    /// Check if a point (screen coordinates) is within this view.
    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        if !self.visible {
            return false;
        }
        let (vx, vy, vw, vh) = self.screen_rect;
        x >= vx && x < vx + vw as i32 && y >= vy && y < vy + vh as i32
    }

    /// Show this view.
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide this view.
    pub fn hide(&mut self) {
        self.visible = false;
        self.focused = false;
    }

    /// Set focus on this view.
    pub fn focus(&mut self) {
        self.focused = true;
        self.ul_view.focus();
    }

    /// Remove focus from this view.
    pub fn unfocus(&mut self) {
        self.focused = false;
        self.ul_view.unfocus();
    }

    /// Fire a mouse event to the underlying Ultralight view.
    pub fn fire_mouse_event(&self, event: MouseEvent) {
        self.ul_view.fire_mouse_event(event);
    }

    /// Fire a keyboard event to the underlying Ultralight view.
    pub fn fire_key_event(&self, event: KeyEvent) {
        self.ul_view.fire_key_event(event);
    }

    /// Fire a scroll event to the underlying Ultralight view.
    pub fn fire_scroll_event(&self, event: ScrollEvent) {
        self.ul_view.fire_scroll_event(event);
    }

    /// Get the screen rect position/size for coordinate transformation.
    pub fn get_screen_rect(&self) -> (i32, i32, u32, u32) {
        self.screen_rect
    }
}

