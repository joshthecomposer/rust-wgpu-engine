//! Slider widget - draggable value control.
//!
//! Features:
//! - Click or drag to set value
//! - Visual track and thumb
//! - Configurable min/max range

use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Color, Rect, Style};

use super::Widget;

/// Default slider height.
const DEFAULT_HEIGHT: f32 = 20.0;
/// Track height (centered in slider).
const TRACK_HEIGHT: f32 = 6.0;
/// Thumb size.
const THUMB_SIZE: f32 = 16.0;

/// A slider widget for numeric value selection.
pub struct Slider {
    pub style: Style,
    /// Minimum value.
    pub min_value: f32,
    /// Maximum value.
    pub max_value: f32,
    /// Current value.
    pub value: f32,
    /// Track background color.
    pub track_color: Color,
    /// Track border color.
    pub track_border_color: Color,
    /// Filled portion of track.
    pub fill_color: Color,
    /// Thumb color.
    pub thumb_color: Color,
    /// Thumb color when pressed.
    pub thumb_pressed_color: Color,

    // Computed state
    rect: Rect,
    is_dragging: bool,
    is_hovered: bool,
}

impl Slider {
    /// Creates a new Slider with the given style.
    pub fn new(style: Style) -> Self {
        Self {
            style,
            min_value: 0.0,
            max_value: 100.0,
            value: 50.0,
            track_color: Color::Rgba(0.0, 0.0, 0.0, 1.0),
            track_border_color: Color::Variable("stone-light".to_string()),
            fill_color: Color::Variable("runic-gold-translucent".to_string()),
            thumb_color: Color::Variable("runic-gold".to_string()),
            thumb_pressed_color: Color::Variable("icon-glow".to_string()),
            rect: Rect::default(),
            is_dragging: false,
            is_hovered: false,
        }
    }

    /// Builder: set value range.
    pub fn with_range(mut self, min: f32, max: f32) -> Self {
        self.min_value = min;
        self.max_value = max;
        self
    }

    /// Builder: set current value.
    pub fn with_value(mut self, value: f32) -> Self {
        self.value = value.clamp(self.min_value, self.max_value);
        self
    }

    /// Builder: set track color.
    pub fn with_track_color(mut self, color: Color) -> Self {
        self.track_color = color;
        self
    }

    /// Builder: set fill color.
    pub fn with_fill_color(mut self, color: Color) -> Self {
        self.fill_color = color;
        self
    }

    /// Builder: set thumb color.
    pub fn with_thumb_color(mut self, color: Color) -> Self {
        self.thumb_color = color;
        self
    }

    /// Get current value.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Set current value.
    pub fn set_value(&mut self, value: f32) {
        self.value = value.clamp(self.min_value, self.max_value);
    }

    /// Get normalized value (0.0 to 1.0).
    fn normalized(&self) -> f32 {
        if (self.max_value - self.min_value).abs() < f32::EPSILON {
            0.0
        } else {
            (self.value - self.min_value) / (self.max_value - self.min_value)
        }
    }

    /// Convert mouse x position to value.
    fn x_to_value(&self, x: f32) -> f32 {
        let track_start = self.rect.x + THUMB_SIZE / 2.0;
        let track_end = self.rect.x + self.rect.width - THUMB_SIZE / 2.0;
        let track_width = track_end - track_start;

        if track_width <= 0.0 {
            return self.min_value;
        }

        let clamped_x = x.clamp(track_start, track_end);
        let normalized = (clamped_x - track_start) / track_width;
        self.min_value + normalized * (self.max_value - self.min_value)
    }

    /// Get thumb rect based on current value.
    fn thumb_rect(&self) -> Rect {
        let normalized = self.normalized();
        let track_start = self.rect.x + THUMB_SIZE / 2.0;
        let track_end = self.rect.x + self.rect.width - THUMB_SIZE / 2.0;
        let thumb_x = track_start + normalized * (track_end - track_start) - THUMB_SIZE / 2.0;
        let thumb_y = self.rect.y + (self.rect.height - THUMB_SIZE) / 2.0;

        Rect::new(thumb_x, thumb_y, THUMB_SIZE, THUMB_SIZE)
    }
}

impl Widget for Slider {
    fn layout(&mut self, _font_system: &mut FontSystem, available: Rect) {
        let (mt, mr, mb, ml) = self
            .style
            .resolve_margins(available.width, available.height);

        let content_x = available.x + ml;
        let content_y = available.y + mt;
        let max_width = available.width - ml - mr;
        let max_height = available.height - mt - mb;

        let width = self.style.width.resolve_or(max_width, 150.0).min(max_width);

        let height = self
            .style
            .height
            .resolve_or(max_height, DEFAULT_HEIGHT)
            .min(max_height);

        self.rect = Rect::new(content_x, content_y, width, height);
    }

    fn update(&mut self, ctx: &mut UiContext) -> bool {
        let mouse_pos = ctx.mouse_pos();

        self.is_hovered = self.rect.contains(mouse_pos);

        if ctx.is_click_start() && self.is_hovered {
            self.is_dragging = true;
            let new_value = self.x_to_value(mouse_pos.x);
            if (new_value - self.value).abs() > 0.01 {
                self.value = new_value;
                return true;
            }
        }

        if self.is_dragging && !ctx.is_mouse_down() {
            self.is_dragging = false;
        }

        if self.is_dragging && ctx.is_mouse_down() {
            let new_value = self.x_to_value(mouse_pos.x);
            if (new_value - self.value).abs() > 0.01 {
                self.value = new_value;
                return true;
            }
        }

        false
    }

    fn render(&self, renderer: &mut UiRenderer) {
        let track_y = self.rect.y + (self.rect.height - TRACK_HEIGHT) / 2.0;

        let track_border_rect = Rect::new(
            self.rect.x + THUMB_SIZE / 2.0 - 1.0,
            track_y - 1.0,
            self.rect.width - THUMB_SIZE + 2.0,
            TRACK_HEIGHT + 2.0,
        );
        renderer.draw_rect(track_border_rect, self.track_border_color.to_rgba(), 0.0);

        let track_rect = Rect::new(
            self.rect.x + THUMB_SIZE / 2.0,
            track_y,
            self.rect.width - THUMB_SIZE,
            TRACK_HEIGHT,
        );
        renderer.draw_rect(track_rect, self.track_color.to_rgba(), 0.0);

        let filled_width = self.normalized() * (self.rect.width - THUMB_SIZE);
        if filled_width > 0.0 {
            let fill_rect = Rect::new(
                self.rect.x + THUMB_SIZE / 2.0,
                track_y,
                filled_width,
                TRACK_HEIGHT,
            );
            renderer.draw_rect(fill_rect, self.fill_color.to_rgba(), 0.0);
        }

        let thumb = self.thumb_rect();
        let thumb_color = if self.is_dragging {
            self.thumb_pressed_color.to_rgba()
        } else {
            self.thumb_color.to_rgba()
        };

        let thumb_border = Rect::new(
            thumb.x - 1.0,
            thumb.y - 1.0,
            thumb.width + 2.0,
            thumb.height + 2.0,
        );
        renderer.draw_rect(thumb_border, [0.0, 0.0, 0.0, 1.0], 0.0);

        renderer.draw_rect(thumb, thumb_color, 0.0);
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
