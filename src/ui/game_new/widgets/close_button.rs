//! CloseButton widget - renders "×" icon with hover color change.
//!
//! Features:
//! - Renders "×" text (20px font size)
//! - Changes color on hover: stone-light → old-text
//! - Handles click events with ID logging
//! - Default size: 20x20px

use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Color, Rect, Style};

use super::Widget;

/// Default font size for close button text.
const CLOSE_FONT_SIZE: f32 = 20.0;
/// Default size for close button.
const DEFAULT_SIZE: f32 = 20.0;

/// A close button widget that renders "×" with hover color change.
pub struct CloseButton {
    pub style: Style,
    /// Normal text color (when not hovered).
    pub normal_color: Color,
    /// Hover text color.
    pub hover_color: Color,

    // Computed state
    rect: Rect,
    is_hovered: bool,
    is_pressed: bool,
    text_size: (f32, f32),
}

impl CloseButton {
    /// Creates a new CloseButton with default styling.
    pub fn new(style: Style) -> Self {
        Self {
            style,
            normal_color: Color::Variable("stone-light".to_string()),
            hover_color: Color::Variable("old-text".to_string()),
            rect: Rect::default(),
            is_hovered: false,
            is_pressed: false,
            text_size: (0.0, 0.0),
        }
    }

    /// Builder: set normal color.
    pub fn with_normal_color(mut self, color: Color) -> Self {
        self.normal_color = color;
        self
    }

    /// Builder: set hover color.
    pub fn with_hover_color(mut self, color: Color) -> Self {
        self.hover_color = color;
        self
    }

    /// Returns true if button is currently hovered.
    pub fn is_hovered(&self) -> bool {
        self.is_hovered
    }

    /// Returns true if button is currently pressed.
    pub fn is_pressed(&self) -> bool {
        self.is_pressed
    }
}

impl Widget for CloseButton {
    fn layout(&mut self, font_system: &mut FontSystem, available: Rect) {
        let (mt, mr, mb, ml) = self
            .style
            .resolve_margins(available.width, available.height);

        let content_x = available.x + ml;
        let content_y = available.y + mt;
        let max_width = available.width - ml - mr;
        let max_height = available.height - mt - mb;

        let width = self
            .style
            .width
            .resolve_or(max_width, DEFAULT_SIZE)
            .min(max_width);

        let height = self
            .style
            .height
            .resolve_or(max_height, DEFAULT_SIZE)
            .min(max_height);

        self.rect = Rect::new(content_x, content_y, width, height);

        let font_size = self.style.font_size.unwrap_or(CLOSE_FONT_SIZE);
        self.text_size = font_system.measure_text("x", font_size, None);
    }

    fn update(&mut self, ctx: &mut UiContext) -> bool {
        let mouse_pos = ctx.mouse_pos();
        self.is_hovered = self.rect.contains(mouse_pos);

        if self.is_hovered {
            if ctx.is_mouse_down() {
                self.is_pressed = true;
            }

            if ctx.is_click_start() {
                return true;
            }
        } else {
            self.is_pressed = false;
        }

        if !ctx.is_mouse_down() {
            self.is_pressed = false;
        }

        false
    }

    fn render(&self, renderer: &mut UiRenderer) {
        let text_color = if self.is_hovered {
            self.hover_color.to_rgba()
        } else {
            self.normal_color.to_rgba()
        };

        let font_size = self.style.font_size.unwrap_or(CLOSE_FONT_SIZE);
        let text_x = self.rect.x + (self.rect.width - self.text_size.0) / 2.0;
        let text_y = self.rect.y + (self.rect.height - self.text_size.1) / 2.0;

        renderer.draw_text(
            "X",
            text_x,
            text_y,
            font_size,
            text_color,
            self.style.font_family.as_deref(),
        );
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn id(&self) -> Option<&str> {
        self.style.id.as_deref()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn find_widget_mut(&mut self, id: &str) -> Option<&mut dyn Widget> {
        if self.id() == Some(id) {
            return Some(self);
        }
        None
    }
}
