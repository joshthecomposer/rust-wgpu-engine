use std::sync::Arc;
use winit::window::{CursorGrabMode, Window};

use crate::enums_types::CursorMode;

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
