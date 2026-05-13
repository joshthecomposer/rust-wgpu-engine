use std::sync::Arc;
use winit::window::{CursorGrabMode, Window};

use crate::enums_types::CursorMode;

/// Physical-pixel size of the winit canvas on the web (CSS size × devicePixelRatio).
/// winit reports a 1×1 `inner_size` for a CSS-sized canvas, so we read the element
/// directly and size the drawing buffer to match the surface (avoids stretching).
#[cfg(target_arch = "wasm32")]
pub fn web_canvas_physical_size(window: &Window) -> (u32, u32) {
    use winit::platform::web::WindowExtWebSys;
    let dpr = web_sys::window()
        .map(|w| w.device_pixel_ratio())
        .filter(|d| *d > 0.0)
        .unwrap_or(1.0);
    if let Some(canvas) = window.canvas() {
        let cw = canvas.client_width();
        let ch = canvas.client_height();
        if cw > 0 && ch > 0 {
            let pw = (((cw as f64) * dpr).round() as u32).max(1);
            let ph = (((ch as f64) * dpr).round() as u32).max(1);
            canvas.set_width(pw);
            canvas.set_height(ph);
            return (pw, ph);
        }
    }
    (1280, 720)
}

pub struct Platform {
    pub fb_width: u32,
    pub fb_height: u32,
    pub window: Option<Arc<Window>>,
}

impl Platform {
    pub fn set_winit_cursor_mode(&mut self, mode: CursorMode) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        match mode {
            CursorMode::Normal => {
                let _ = window.set_cursor_grab(CursorGrabMode::None);
                window.set_cursor_visible(true);
            }
            CursorMode::Hidden => {
                let _ = window.set_cursor_grab(CursorGrabMode::Locked);
                window.set_cursor_visible(false);
            }
            CursorMode::Disabled => {
                let _ = window.set_cursor_grab(CursorGrabMode::Locked);
                window.set_cursor_visible(false);
            }
        }
    }
}
