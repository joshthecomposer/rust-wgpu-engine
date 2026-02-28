//! TooltipManager widget - manages and renders tooltips for UI elements.
//!
//! The TooltipManager is a special widget that renders tooltip popups
//! above other UI content. It should be rendered last in the UI hierarchy
//! to ensure tooltips appear on top of all other elements.
//!
//! # Usage Pattern
//! ```rust
//! // During update phase, check if any widget wants to show a tooltip
//! if let Some((name, desc)) = ability_slot.get_tooltip_info() {
//!     tooltip_manager.show(name, desc, slot_rect);
//! }
//!
//! // During render phase, render the tooltip last
//! tooltip_manager.render(renderer);
//! ```

use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Color, Rect, Style};

use super::Widget;

/// Padding inside the tooltip box.
const TOOLTIP_PADDING: f32 = 8.0;
/// Font size for tooltip title.
const TITLE_FONT_SIZE: f32 = 14.0;
/// Font size for tooltip description.
const DESC_FONT_SIZE: f32 = 12.0;
/// Gap between title and description.
const TITLE_DESC_GAP: f32 = 4.0;

/// Manages tooltip display for UI widgets.
///
/// TooltipManager is designed to be used as a singleton at the root of your UI.
/// Widgets that want to display tooltips should call `show()` during their update phase,
/// and the TooltipManager should be rendered after all other UI elements.
pub struct TooltipManager {
    pub style: Style,
    rect: Rect,

    // Tooltip state
    is_visible: bool,
    title: String,
    description: String,
    /// Position where the tooltip should appear (usually above the hovered widget).
    anchor_rect: Rect,

    // Styling
    pub background_color: Color,
    pub border_color: Color,
    pub title_color: Color,
    pub description_color: Color,

    // Cached layout info
    tooltip_rect: Rect,
    title_pos: (f32, f32),
    desc_pos: (f32, f32),
}

impl TooltipManager {
    /// Creates a new TooltipManager with default styling.
    pub fn new(style: Style) -> Self {
        Self {
            style,
            rect: Rect::default(),
            is_visible: false,
            title: String::new(),
            description: String::new(),
            anchor_rect: Rect::default(),
            background_color: Color::Rgba(0.06, 0.09, 0.16, 0.95), // deep-void
            border_color: Color::Rgba(0.28, 0.33, 0.42, 1.0),      // stone-light
            title_color: Color::Rgba(0.85, 0.47, 0.02, 1.0),       // runic-gold
            description_color: Color::Rgba(0.58, 0.64, 0.72, 1.0), // old-text
            tooltip_rect: Rect::default(),
            title_pos: (0.0, 0.0),
            desc_pos: (0.0, 0.0),
        }
    }

    /// Show a tooltip at the specified anchor position.
    ///
    /// The tooltip will be positioned above the anchor rect.
    /// Call this during the update phase of widgets that want to show tooltips.
    pub fn show(&mut self, title: &str, description: &str, anchor_rect: Rect) {
        self.is_visible = true;
        self.title = title.to_string();
        self.description = description.to_string();
        self.anchor_rect = anchor_rect;
    }

    /// Hide the current tooltip.
    pub fn hide(&mut self) {
        self.is_visible = false;
    }

    /// Clear tooltip state at the start of each frame.
    /// Call this at the beginning of the update cycle.
    pub fn begin_frame(&mut self) {
        self.is_visible = false;
    }

    /// Calculate tooltip dimensions and position based on content.
    fn calculate_layout(&mut self, font_system: &mut FontSystem) {
        if !self.is_visible {
            return;
        }

        // Measure text dimensions
        let title_size = font_system.measure_text(&self.title, TITLE_FONT_SIZE, None);
        let desc_size = if self.description.is_empty() {
            (0.0, 0.0)
        } else {
            font_system.measure_text(&self.description, DESC_FONT_SIZE, None)
        };

        // Calculate tooltip size
        let content_width = title_size.0.max(desc_size.0);
        let content_height = if self.description.is_empty() {
            title_size.1
        } else {
            title_size.1 + TITLE_DESC_GAP + desc_size.1
        };
        // calculate tooltip size (add accent bar width: 4px + 4px spacing)
        let accent_width = 4.0 + 4.0; // accent bar + spacing
        let tooltip_width = content_width + TOOLTIP_PADDING * 2.0 + accent_width;
        let tooltip_height = content_height + TOOLTIP_PADDING * 2.0;

        // Position above the anchor, aligned to left edge
        let tooltip_x = self.anchor_rect.x;
        let tooltip_y = self.anchor_rect.y - tooltip_height - 8.0;

        self.tooltip_rect = Rect::new(tooltip_x, tooltip_y, tooltip_width, tooltip_height);

        // Calculate text positions
        self.title_pos = (
            self.tooltip_rect.x + TOOLTIP_PADDING,
            self.tooltip_rect.y + TOOLTIP_PADDING,
        );
        self.desc_pos = (
            self.tooltip_rect.x + TOOLTIP_PADDING,
            self.tooltip_rect.y + TOOLTIP_PADDING + title_size.1 + TITLE_DESC_GAP,
        );
    }
}

impl Widget for TooltipManager {
    fn layout(&mut self, font_system: &mut FontSystem, available: Rect) {
        // TooltipManager takes up the full available space (it's an overlay)
        self.rect = available;

        // Calculate tooltip position and size
        self.calculate_layout(font_system);
    }

    fn update(&mut self, _ctx: &mut UiContext) -> bool {
        // TooltipManager doesn't consume input events
        false
    }

    fn render(&self, renderer: &mut UiRenderer) {
        if !self.is_visible {
            return;
        }

        let border_radius = 0.0; // No rounded corners for sharp design
        let border_width = 1.0;
        let accent_width = 4.0;
        let shadow_offset = 4.0;

        // Draw drop shadow (offset 4px right and down, 50% opacity black)
        let shadow_rect = Rect::new(
            self.tooltip_rect.x + shadow_offset,
            self.tooltip_rect.y + shadow_offset,
            self.tooltip_rect.width,
            self.tooltip_rect.height,
        );
        let shadow_color = [0.0, 0.0, 0.0, 0.5];
        renderer.draw_rect(shadow_rect, shadow_color, border_radius);

        // Draw border (outer)
        let border_color = self.border_color.to_rgba();
        renderer.draw_rect(self.tooltip_rect, border_color, border_radius);

        // Draw background (inner)
        let bg_rect = self.tooltip_rect.shrink(border_width);
        let bg_color = self.background_color.to_rgba();
        renderer.draw_rect(bg_rect, bg_color, border_radius);

        // Draw left accent bar (runic-gold, 4px wide)
        let accent_rect = Rect::new(
            self.tooltip_rect.x + border_width,
            self.tooltip_rect.y + border_width,
            accent_width,
            self.tooltip_rect.height - border_width * 2.0,
        );
        let accent_color = [0.85, 0.68, 0.42, 1.0]; // runic-gold
        renderer.draw_rect(accent_rect, accent_color, 0.0);

        // Adjust text positions to account for accent bar
        let text_offset_x = accent_width + 4.0; // Accent width + spacing

        // Draw title
        let title_color = self.title_color.to_rgba();
        renderer.draw_text(
            &self.title,
            self.title_pos.0 + text_offset_x,
            self.title_pos.1,
            TITLE_FONT_SIZE,
            title_color,
            None,
        );

        // Draw description (if present)
        if !self.description.is_empty() {
            let desc_color = self.description_color.to_rgba();
            renderer.draw_text(
                &self.description,
                self.desc_pos.0 + text_offset_x,
                self.desc_pos.1,
                DESC_FONT_SIZE,
                desc_color,
                None,
            );
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
