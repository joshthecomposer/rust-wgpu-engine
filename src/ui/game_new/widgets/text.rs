use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Rect, Style};

use super::Widget;

/// A widget that renders a string of text with a specific style.
///
/// The Text widget supports basic styling like color, font size, and margins.
/// It measures its content during the layout phase to determine its size.
pub struct Text {
    /// The text content to display.
    pub content: String,
    /// The style definitions for this widget.
    pub style: Style,
    /// The computed layout bounds of the widget.
    rect: Rect,
}

impl Text {
    /// Creates a new Text widget.
    pub fn new(content: String, style: Style) -> Self {
        Self {
            content,
            style,
            rect: Rect::default(),
        }
    }
}

use crate::ui::game_new::font_system::FontSystem;

impl Widget for Text {
    fn layout(&mut self, font_system: &mut FontSystem, available: Rect) {
        let (margin_top, margin_right, margin_bottom, margin_left) = self
            .style
            .resolve_margins(available.width, available.height);

        let content_start_x = available.x + margin_left;
        let content_start_y = available.y + margin_top;

        let max_available_width = available.width - margin_left - margin_right;
        let max_available_height = available.height - margin_top - margin_bottom;

        let font_size = self.style.font_size.unwrap_or(16.0);
        let (measured_width, measured_height) = font_system.measure_text(&self.content, font_size);

        // determine final dimensions: prefer explicit style size, fallback to measured text size.
        let width = self
            .style
            .width
            .resolve_or(measured_width, max_available_width);
        let height = self
            .style
            .height
            .resolve_or(measured_height, max_available_height);

        let clipped_width = width.min(max_available_width);
        let clipped_height = height.min(max_available_height);

        self.rect = Rect::new(
            content_start_x,
            content_start_y,
            clipped_width,
            clipped_height,
        );
    }

    fn update(&mut self, _ctx: &mut UiContext) -> bool {
        false
    }

    fn render(&self, renderer: &mut UiRenderer) {
        let bg_color = self.style.background.to_rgba();
        if bg_color[3] > 0.0 {
            renderer.draw_rect(self.rect, bg_color);
        }

        if let Some(color) = &self.style.color {
            let font_size = self.style.font_size.unwrap_or(16.0);

            // Render text at the top-left of the computed rect.
            renderer.draw_text(
                &self.content,
                self.rect.x,
                self.rect.y,
                font_size,
                color.to_rgba(),
            );
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }
}
