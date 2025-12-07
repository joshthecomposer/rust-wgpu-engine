#![allow(dead_code)]

use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Mouse button types for Ultralight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UltralightMouseButton {
    Left,
    Right,
    Middle,
}

/// Input events to be dispatched to Ultralight views.
#[derive(Debug, Clone)]
pub enum UltralightInputEvent {
    MouseMoved { x: i32, y: i32 },
    MouseDown { x: i32, y: i32, button: UltralightMouseButton },
    MouseUp { x: i32, y: i32, button: UltralightMouseButton },
    Scroll { dx: i32, dy: i32 },
    KeyDown { key: u32, modifiers: Modifiers },
    KeyUp { key: u32, modifiers: Modifiers },
    TextInput { character: char },
}

/// Adapter for converting winit input events to Ultralight format.
pub struct InputAdapter {
    /// Current mouse position
    mouse_position: (f64, f64),
    /// Mouse buttons currently held
    mouse_buttons: [bool; 3], // Left, Right, Middle
    /// Modifier keys state
    modifiers: Modifiers,
}

/// Modifier key state.
#[derive(Debug, Default, Clone, Copy)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool, // Windows/Command key
}

impl Default for InputAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl InputAdapter {
    pub fn new() -> Self {
        Self {
            mouse_position: (0.0, 0.0),
            mouse_buttons: [false; 3],
            modifiers: Modifiers::default(),
        }
    }

    /// Get current mouse position.
    pub fn mouse_position(&self) -> (f64, f64) {
        self.mouse_position
    }

    /// Check if a mouse button is pressed.
    pub fn is_mouse_button_pressed(&self, button: usize) -> bool {
        self.mouse_buttons.get(button).copied().unwrap_or(false)
    }

    /// Get current modifier state.
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Process a winit WindowEvent and return Ultralight events to dispatch.
    ///
    /// Returns true if the event was handled and should not be passed to the game.
    pub fn process_event(&mut self, event: &WindowEvent) -> Vec<UltralightInputEvent> {
        let mut events = Vec::new();

        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = (position.x, position.y);
                events.push(UltralightInputEvent::MouseMoved {
                    x: position.x as i32,
                    y: position.y as i32,
                });
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let btn_index = match button {
                    MouseButton::Left => 0,
                    MouseButton::Right => 1,
                    MouseButton::Middle => 2,
                    _ => return events,
                };

                let pressed = *state == ElementState::Pressed;
                self.mouse_buttons[btn_index] = pressed;

                let ul_button = match button {
                    MouseButton::Left => UltralightMouseButton::Left,
                    MouseButton::Right => UltralightMouseButton::Right,
                    MouseButton::Middle => UltralightMouseButton::Middle,
                    _ => return events,
                };

                if pressed {
                    events.push(UltralightInputEvent::MouseDown {
                        x: self.mouse_position.0 as i32,
                        y: self.mouse_position.1 as i32,
                        button: ul_button,
                    });
                } else {
                    events.push(UltralightInputEvent::MouseUp {
                        x: self.mouse_position.0 as i32,
                        y: self.mouse_position.1 as i32,
                        button: ul_button,
                    });
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        (*x as i32 * 40, *y as i32 * 40)
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        (pos.x as i32, pos.y as i32)
                    }
                };
                events.push(UltralightInputEvent::Scroll { dx, dy });
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key_code) = event.physical_key {
                    let ul_key = self.map_key_code(key_code);
                    let pressed = event.state == ElementState::Pressed;

                    // Update modifier state
                    match key_code {
                        KeyCode::ShiftLeft | KeyCode::ShiftRight => self.modifiers.shift = pressed,
                        KeyCode::ControlLeft | KeyCode::ControlRight => {
                            self.modifiers.ctrl = pressed
                        }
                        KeyCode::AltLeft | KeyCode::AltRight => self.modifiers.alt = pressed,
                        KeyCode::SuperLeft | KeyCode::SuperRight => self.modifiers.meta = pressed,
                        _ => {}
                    }

                    if pressed {
                        events.push(UltralightInputEvent::KeyDown {
                            key: ul_key,
                            modifiers: self.modifiers,
                        });

                        // Also send text input for printable characters
                        if let Some(text) = &event.text {
                            for c in text.chars() {
                                events.push(UltralightInputEvent::TextInput { character: c });
                            }
                        }
                    } else {
                        events.push(UltralightInputEvent::KeyUp {
                            key: ul_key,
                            modifiers: self.modifiers,
                        });
                    }
                }
            }

            _ => {}
        }

        events
    }

    /// Map winit KeyCode to Ultralight virtual key code (Windows VK codes).
    fn map_key_code(&self, key: KeyCode) -> u32 {
        // Ultralight uses Windows virtual key codes
        match key {
            KeyCode::Backspace => 0x08,
            KeyCode::Tab => 0x09,
            KeyCode::Enter => 0x0D,
            KeyCode::Escape => 0x1B,
            KeyCode::Space => 0x20,
            // Arrow keys
            KeyCode::ArrowLeft => 0x25,
            KeyCode::ArrowUp => 0x26,
            KeyCode::ArrowRight => 0x27,
            KeyCode::ArrowDown => 0x28,
            // Number keys (top row)
            KeyCode::Digit0 => 0x30,
            KeyCode::Digit1 => 0x31,
            KeyCode::Digit2 => 0x32,
            KeyCode::Digit3 => 0x33,
            KeyCode::Digit4 => 0x34,
            KeyCode::Digit5 => 0x35,
            KeyCode::Digit6 => 0x36,
            KeyCode::Digit7 => 0x37,
            KeyCode::Digit8 => 0x38,
            KeyCode::Digit9 => 0x39,
            // Numpad
            KeyCode::Numpad0 => 0x60,
            KeyCode::Numpad1 => 0x61,
            KeyCode::Numpad2 => 0x62,
            KeyCode::Numpad3 => 0x63,
            KeyCode::Numpad4 => 0x64,
            KeyCode::Numpad5 => 0x65,
            KeyCode::Numpad6 => 0x66,
            KeyCode::Numpad7 => 0x67,
            KeyCode::Numpad8 => 0x68,
            KeyCode::Numpad9 => 0x69,
            KeyCode::NumpadDecimal => 0x6E,
            KeyCode::NumpadAdd => 0x6B,
            KeyCode::NumpadSubtract => 0x6D,
            // Special keys
            KeyCode::Delete => 0x2E,
            KeyCode::Home => 0x24,
            KeyCode::End => 0x23,
            // Period and minus (for decimal numbers)
            KeyCode::Period => 0xBE,
            KeyCode::Minus => 0xBD,
            _ => 0,
        }
    }
}

