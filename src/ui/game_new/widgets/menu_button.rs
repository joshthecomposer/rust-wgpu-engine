//! MenuButton widget - styled button with hover accent for menu items.
//!
//! Features:
//! - Normal and hover background colors
//! - Dot bullets (•) on left/right sides when hovered
//! - Centered text label
//! - Click detection with ID logging

use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Color, Rect, Style};

use super::Widget;

/// Default font size for button text.
const BUTTON_FONT_SIZE: f32 = 14.0;
/// Font size for diamond bullets.
const DIAMOND_FONT_SIZE: f32 = 14.0;
/// Left diamond X position offset.
const LEFT_DIAMOND_X: f32 = 12.0;
/// Right diamond X position offset from right edge.
const RIGHT_DIAMOND_X_OFFSET: f32 = 20.0;

/// A menu button with hover accent styling.
pub struct MenuButton {
    pub style: Style,
    /// Button label text.
    pub text: String,
    /// Background color when not hovered.
    pub normal_background: Color,
    /// Background color when hovered.
    pub hover_background: Color,
    /// Left accent bar color (shown on hover).
    pub accent_color: Color,
    /// Border color around the button.
    pub border_color: Color,
    /// Text color for the label.
    pub text_color: Color,
    /// Shadow color (set alpha to 0 to disable shadow).
    pub shadow_color: Color,
    /// Shadow horizontal offset in pixels.
    pub shadow_offset_x: f32,
    /// Shadow vertical offset in pixels.
    pub shadow_offset_y: f32,

    // Computed state
    rect: Rect,
    is_hovered: bool,
    is_pressed: bool,
    text_size: (f32, f32),
}

impl MenuButton {
    /// Creates a new MenuButton with the given text and default styling.
    pub fn new(text: impl Into<String>, style: Style) -> Self {
        Self {
            style,
            text: text.into(),
            normal_background: Color::Variable("stone-dark".to_string()),
            hover_background: Color::Variable("stone-mild".to_string()),
            accent_color: Color::Variable("runic-gold".to_string()),
            border_color: Color::Variable("stone-light".to_string()),
            text_color: Color::Variable("text-light".to_string()),
            shadow_color: Color::Rgba(0.0, 0.0, 0.0, 0.8),
            shadow_offset_x: 3.0,
            shadow_offset_y: 3.0,
            rect: Rect::default(),
            is_hovered: false,
            is_pressed: false,
            text_size: (0.0, 0.0),
        }
    }

    /// Builder: set normal background color.
    pub fn with_normal_background(mut self, color: Color) -> Self {
        self.normal_background = color;
        self
    }

    /// Builder: set hover background color.
    pub fn with_hover_background(mut self, color: Color) -> Self {
        self.hover_background = color;
        self
    }

    /// Builder: set accent color (left bar on hover).
    pub fn with_accent_color(mut self, color: Color) -> Self {
        self.accent_color = color;
        self
    }

    /// Builder: set border color.
    pub fn with_border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    /// Builder: set text color.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Builder: set shadow color (set alpha to 0 to disable shadow).
    pub fn with_shadow_color(mut self, color: Color) -> Self {
        self.shadow_color = color;
        self
    }

    /// Builder: set shadow offset.
    pub fn with_shadow_offset(mut self, x: f32, y: f32) -> Self {
        self.shadow_offset_x = x;
        self.shadow_offset_y = y;
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

impl Widget for MenuButton {
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
            .resolve_or(max_width, max_width)
            .min(max_width);

        let height = self
            .style
            .height
            .resolve_or(max_height, 40.0)
            .min(max_height);

        self.rect = Rect::new(content_x, content_y, width, height);

        // measure text for centering
        let font_size = self.style.font_size.unwrap_or(BUTTON_FONT_SIZE);
        self.text_size = font_system.measure_text(&self.text, font_size, None);
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
        let border_radius = self.style.border_radius;
        let border_width = 1.0;

        let shadow_rgba = self.shadow_color.to_rgba();
        if shadow_rgba[3] > 0.0 {
            let shadow_rect = Rect::new(
                self.rect.x + self.shadow_offset_x,
                self.rect.y + self.shadow_offset_y,
                self.rect.width,
                self.rect.height,
            );
            renderer.draw_rect(shadow_rect, shadow_rgba, border_radius);
        }

        let bg_color = if self.is_pressed {
            let c = self.hover_background.to_rgba();
            [c[0] * 0.8, c[1] * 0.8, c[2] * 0.8, c[3]]
        } else if self.is_hovered {
            self.hover_background.to_rgba()
        } else {
            self.normal_background.to_rgba()
        };

        renderer.draw_rect(self.rect, self.border_color.to_rgba(), border_radius);

        let bg_rect = self.rect.shrink(border_width);
        let inner_radius = (border_radius - border_width).max(0.0);
        if bg_rect.width > 0.0 && bg_rect.height > 0.0 {
            renderer.draw_rect(bg_rect, bg_color, inner_radius);
        }

        if self.is_hovered {
            let diamond_color = self.accent_color.to_rgba();
            let diamond_size = 8.0;

            let left_diamond_x = self.rect.x + LEFT_DIAMOND_X;
            let left_diamond_y = self.rect.y + (self.rect.height - diamond_size) / 2.0 + 4.0;
            let left_diamond_rect = crate::ui::game_new::styles::Rect::new(
                left_diamond_x,
                left_diamond_y,
                diamond_size,
                diamond_size,
            );
            renderer.draw_diamond(left_diamond_rect, diamond_color);

            let right_diamond_x = self.rect.x + self.rect.width - RIGHT_DIAMOND_X_OFFSET;
            let right_diamond_y = self.rect.y + (self.rect.height - diamond_size) / 2.0 + 4.0;
            let right_diamond_rect = crate::ui::game_new::styles::Rect::new(
                right_diamond_x,
                right_diamond_y,
                diamond_size,
                diamond_size,
            );
            renderer.draw_diamond(right_diamond_rect, diamond_color);
        }

        let font_size = self.style.font_size.unwrap_or(BUTTON_FONT_SIZE);
        let text_x = self.rect.x + (self.rect.width - self.text_size.0) / 2.0;
        let text_y = self.rect.y + (self.rect.height - self.text_size.1) / 2.0;

        renderer.draw_text(
            &self.text,
            text_x,
            text_y,
            font_size,
            self.text_color.to_rgba(),
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
