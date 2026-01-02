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
    /// The measured size of the text content
    text_size: (f32, f32),
}

impl Text {
    /// Creates a new Text widget.
    pub fn new(content: String, style: Style) -> Self {
        Self {
            content,
            style,
            rect: Rect::default(),
            text_size: (0.0, 0.0),
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
        self.text_size =
            font_system.measure_text(&self.content, font_size, self.style.font_family.as_deref());
        let (measured_width, measured_height) = self.text_size;

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

    fn update(&mut self, ctx: &mut UiContext) -> bool {
        if self.rect.contains(ctx.mouse_pos()) {
            if ctx.is_click_start() {
                if let Some(id) = &self.style.id {
                    println!("[Text] Clicked. ID: {}", id);
                }
                return true;
            }
        }
        false
    }

    fn render(&self, renderer: &mut UiRenderer) {
        let bg_color = self.style.background.to_rgba();
        if bg_color[3] > 0.0 {
            renderer.draw_rect(self.rect, bg_color);
        }

        if let Some(color) = &self.style.color {
            let font_size = self.style.font_size.unwrap_or(16.0);

            let align = self
                .style
                .text_align
                .unwrap_or(crate::ui::game_new::styles::Alignment::Start);
            let x_offset = match align {
                crate::ui::game_new::styles::Alignment::Start => 0.0,
                crate::ui::game_new::styles::Alignment::Center => {
                    (self.rect.width - self.text_size.0) / 2.0
                }
                crate::ui::game_new::styles::Alignment::End => self.rect.width - self.text_size.0,
                _ => 0.0,
            };

            // Render text at the top-left of the computed rect.
            renderer.draw_text(
                &self.content,
                self.rect.x + x_offset,
                self.rect.y,
                font_size,
                color.to_rgba(),
                self.style.font_family.clone(),
            );
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }
}
