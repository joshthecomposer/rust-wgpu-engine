//! Checkbox widget - toggle control for boolean settings.
//!
//! Features:
//! - Click to toggle on/off
//! - Hover highlighting
//! - Visual checkmark indicator when checked

use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Color, Rect, Style};

use super::Widget;

/// Default checkbox size.
const CHECKBOX_SIZE: f32 = 18.0;
/// Inner checkmark inset.
const CHECKMARK_INSET: f32 = 4.0;

/// A checkbox toggle widget.
pub struct Checkbox {
    pub style: Style,
    /// Whether the checkbox is checked.
    pub checked: bool,
    /// Background color when unchecked.
    pub background_color: Color,
    /// Border color (changes on hover).
    pub border_color: Color,
    /// Border color when hovered.
    pub hover_border_color: Color,
    /// Fill color when checked.
    pub check_color: Color,

    // Computed state
    rect: Rect,
    is_hovered: bool,
}

impl Checkbox {
    /// Creates a new Checkbox with the given style.
    pub fn new(style: Style) -> Self {
        Self {
            style,
            checked: false,
            background_color: Color::Variable("stone-dark".to_string()),
            border_color: Color::Variable("stone-light".to_string()),
            hover_border_color: Color::Variable("runic-gold".to_string()),
            check_color: Color::Variable("runic-gold".to_string()),
            rect: Rect::default(),
            is_hovered: false,
        }
    }

    /// Builder: set checked state.
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Builder: set background color.
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Builder: set border color.
    pub fn with_border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    /// Builder: set hover border color.
    pub fn with_hover_border_color(mut self, color: Color) -> Self {
        self.hover_border_color = color;
        self
    }

    /// Builder: set check fill color.
    pub fn with_check_color(mut self, color: Color) -> Self {
        self.check_color = color;
        self
    }

    /// Returns whether the checkbox is checked.
    pub fn is_checked(&self) -> bool {
        self.checked
    }

    /// Set the checked state.
    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }

    /// Toggle the checked state.
    pub fn toggle(&mut self) {
        self.checked = !self.checked;
    }
}

impl Widget for Checkbox {
    fn layout(&mut self, _font_system: &mut FontSystem, available: Rect) {
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
            .resolve_or(max_width, CHECKBOX_SIZE)
            .min(max_width);

        let height = self
            .style
            .height
            .resolve_or(max_height, CHECKBOX_SIZE)
            .min(max_height);

        self.rect = Rect::new(content_x, content_y, width, height);
    }

    fn update(&mut self, ctx: &mut UiContext) -> bool {
        let mouse_pos = ctx.mouse_pos();

        self.is_hovered = self.rect.contains(mouse_pos);

        if ctx.is_click_start() && self.is_hovered {
            self.toggle();
            if let Some(id) = &self.style.id {
                println!("[Checkbox] Toggled. ID: {}, checked: {}", id, self.checked);
            }
            return true;
        }

        false
    }

    fn render(&self, renderer: &mut UiRenderer) {
        let border_width = 1.0;

        let border_color = if self.is_hovered {
            self.hover_border_color.to_rgba()
        } else {
            self.border_color.to_rgba()
        };
        renderer.draw_rect(self.rect, border_color, 0.0);

        let bg_rect = self.rect.shrink(border_width);
        renderer.draw_rect(bg_rect, self.background_color.to_rgba(), 0.0);

        if self.checked {
            let check_rect = Rect::new(
                self.rect.x + CHECKMARK_INSET,
                self.rect.y + CHECKMARK_INSET,
                self.rect.width - 2.0 * CHECKMARK_INSET,
                self.rect.height - 2.0 * CHECKMARK_INSET,
            );
            renderer.draw_rect(check_rect, self.check_color.to_rgba(), 0.0);
        }
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
