use glam::Vec2;
use winit::event::MouseButton;

use crate::{input::InputState, ui::message_queue::MessageQueue};

pub struct UiContext<'a> {
    pub input: &'a InputState,
    pub messages: &'a mut MessageQueue,
}

impl<'a> UiContext<'a> {
    /// Helper: Where is the mouse right now?
    pub fn mouse_pos(&self) -> Vec2 {
        self.input.mouse_pos_current
    }

    /// Helper: Was the Left Mouse Button clicked *just now*?
    /// Logic: It is Down this frame AND Up last frame.
    pub fn is_click_start(&self) -> bool {
        let btn = MouseButton::Left;
        self.input.mouse_current.contains(&btn) && !self.input.mouse_previous.contains(&btn)
    }

    /// Helper: Is the Left Mouse Button being held down?
    pub fn is_mouse_down(&self) -> bool {
        self.input.mouse_current.contains(&MouseButton::Left)
    }

    /// Helper: Get mouse scroll wheel delta (y is vertical: positive = scroll up/away)
    pub fn scroll_delta(&self) -> Vec2 {
        self.input.scroll_delta
    }
}
