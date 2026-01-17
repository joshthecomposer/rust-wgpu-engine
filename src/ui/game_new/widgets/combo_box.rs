//! ComboBox widget - dropdown select with expandable options list.
//!
//! Features:
//! - Displays currently selected option
//! - Expandable dropdown list on click
//! - Hover highlighting for options
//! - Click to select and close

use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Color, Rect, Style};

use super::Widget;

/// Default item height in dropdown.
const ITEM_HEIGHT: f32 = 28.0;
/// Default font size for combo box text.
const COMBO_FONT_SIZE: f32 = 12.0;
/// Arrow indicator size.
const ARROW_SIZE: f32 = 8.0;
/// Padding inside the combo box button.
const PADDING: f32 = 8.0;

/// A dropdown select widget.
pub struct ComboBox {
    pub style: Style,
    /// List of selectable options.
    pub options: Vec<String>,
    /// Index of currently selected option.
    pub selected_index: usize,
    /// Placeholder text when nothing selected.
    pub placeholder: String,
    /// Background color of the dropdown.
    pub dropdown_background: Color,
    /// Background color when hovering an option.
    pub item_hover_color: Color,
    /// Text color.
    pub text_color: Color,
    /// Border color.
    pub border_color: Color,
    /// Height of each item in dropdown.
    pub item_height: f32,

    // Computed state
    rect: Rect,
    dropdown_rect: Rect,
    is_open: bool,
    is_hovered: bool,
    hovered_option: Option<usize>,
}

impl ComboBox {
    /// Creates a new ComboBox with the given options and style.
    pub fn new(options: Vec<String>, style: Style) -> Self {
        Self {
            style,
            options,
            selected_index: 0,
            placeholder: String::from("Select..."),
            dropdown_background: Color::Variable("stone-dark".to_string()),
            item_hover_color: Color::Variable("stone-mild".to_string()),
            text_color: Color::Variable("text-light".to_string()),
            border_color: Color::Variable("stone-light".to_string()),
            item_height: ITEM_HEIGHT,
            rect: Rect::default(),
            dropdown_rect: Rect::default(),
            is_open: false,
            is_hovered: false,
            hovered_option: None,
        }
    }

    /// Builder: set selected index.
    pub fn with_selected_index(mut self, index: usize) -> Self {
        self.selected_index = index.min(self.options.len().saturating_sub(1));
        self
    }

    /// Builder: set placeholder text.
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Builder: set dropdown background color.
    pub fn with_dropdown_background(mut self, color: Color) -> Self {
        self.dropdown_background = color;
        self
    }

    /// Builder: set item hover color.
    pub fn with_item_hover_color(mut self, color: Color) -> Self {
        self.item_hover_color = color;
        self
    }

    /// Builder: set text color.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Builder: set border color.
    pub fn with_border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    /// Returns the currently selected option text.
    pub fn selected_text(&self) -> &str {
        if self.options.is_empty() {
            &self.placeholder
        } else {
            self.options
                .get(self.selected_index)
                .map(|s| s.as_str())
                .unwrap_or(&self.placeholder)
        }
    }

    /// Returns true if dropdown is currently open.
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Programmatically close the dropdown.
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Get the selected index.
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Set the selected index.
    pub fn set_selected_index(&mut self, index: usize) {
        self.selected_index = index.min(self.options.len().saturating_sub(1));
    }
}

impl Widget for ComboBox {
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
            .resolve_or(max_width, max_width)
            .min(max_width);

        let height = self
            .style
            .height
            .resolve_or(max_height, 32.0)
            .min(max_height);

        let font_size = self.style.font_size.unwrap_or(COMBO_FONT_SIZE);
        let mut max_text_width = 0.0_f32;
        for opt in &self.options {
            let (measured_w, _) =
                _font_system.measure_text(opt, font_size, self.style.font_family.as_deref());
            max_text_width = max_text_width.max(measured_w);
        }

        let arrow_space = ARROW_SIZE + 12.0; // gap between text and arrow
        let needed_button_width = max_text_width + PADDING * 2.0 + arrow_space;

        let width = width.max(needed_button_width).min(max_width);

        self.rect = Rect::new(content_x, content_y, width, height);

        let dropdown_height = self.options.len() as f32 * self.item_height;

        let mut dropdown_width = width;
        for opt in &self.options {
            let (measured_w, _) =
                _font_system.measure_text(opt, font_size, self.style.font_family.as_deref());
            let needed = measured_w + PADDING * 2.0;
            if needed > dropdown_width {
                dropdown_width = needed;
            }
        }

        let mut dropdown_x = self.rect.x;
        let right_bound = content_x + max_width;
        if dropdown_width <= max_width {
            if dropdown_x + dropdown_width > right_bound {
                dropdown_x = (right_bound - dropdown_width).max(content_x);
            }
        } else {
            dropdown_x = self.rect.x;
        }

        self.dropdown_rect = Rect::new(
            dropdown_x,
            self.rect.y + self.rect.height,
            dropdown_width,
            dropdown_height,
        );
    }

    fn update(&mut self, ctx: &mut UiContext) -> bool {
        let mouse_pos = ctx.mouse_pos();

        self.is_hovered = self.rect.contains(mouse_pos);

        self.hovered_option = None;
        if self.is_open {
            for (i, _) in self.options.iter().enumerate() {
                let item_rect = Rect::new(
                    self.dropdown_rect.x,
                    self.dropdown_rect.y + i as f32 * self.item_height,
                    self.dropdown_rect.width,
                    self.item_height,
                );
                if item_rect.contains(mouse_pos) {
                    self.hovered_option = Some(i);
                    break;
                }
            }
        }

        if ctx.is_click_start() {
            // ! PRIORITY 1: If dropdown is open, handle dropdown interactions first
            // ! This prevents clicks on dropdown items from hitting widgets underneath
            if self.is_open {
                if let Some(index) = self.hovered_option {
                    self.selected_index = index;
                    self.is_open = false;
                    if let Some(id) = &self.style.id {
                        println!("[ComboBox] Selected index {}. ID: {}", index, id);
                    }
                    return true;
                }

                if self.dropdown_rect.contains(mouse_pos) {
                    return true;
                }

                if self.rect.contains(mouse_pos) {
                    self.is_open = false;
                    if let Some(id) = &self.style.id {
                        println!("[ComboBox] Toggled. ID: {}, open: {}", id, self.is_open);
                    }
                    return true;
                }

                self.is_open = false;
                return true;
            }

            // ! PRIORITY 2: Dropdown is closed - check for click on main button to open
            if self.rect.contains(mouse_pos) {
                self.is_open = true;
                if let Some(id) = &self.style.id {
                    println!("[ComboBox] Toggled. ID: {}, open: {}", id, self.is_open);
                }
                return true;
            }
        }

        if self.is_open && self.dropdown_rect.contains(mouse_pos) {
            return true;
        }

        false
    }

    fn render(&self, renderer: &mut UiRenderer) {
        let border_radius = self.style.border_radius;
        let border_width = 1.0;

        renderer.draw_rect(self.rect, self.border_color.to_rgba(), border_radius);

        let bg_rect = self.rect.shrink(border_width);
        let inner_radius = (border_radius - border_width).max(0.0);
        let bg_color = if self.is_hovered || self.is_open {
            self.item_hover_color.to_rgba()
        } else {
            self.dropdown_background.to_rgba()
        };
        renderer.draw_rect(bg_rect, bg_color, inner_radius);

        let font_size = self.style.font_size.unwrap_or(COMBO_FONT_SIZE);
        let text_x = self.rect.x + PADDING;
        let text_y = self.rect.y + (self.rect.height - font_size) / 2.0;

        // reserve more space on the right for the arrow so glyphs are clipped before the arrow
        // increased to avoid labels like "16x MSAA" being cut off.
        let reserved = ARROW_SIZE + 12.0;
        let clip_width = (self.rect.width - PADDING * 2.0 - reserved).max(0.0);
        let text_clip = Rect::new(text_x, self.rect.y, clip_width, self.rect.height);
        renderer.push_scissor(text_clip);

        // draw selected text in the widget's text color (keep white)
        renderer.draw_text(
            self.selected_text(),
            text_x,
            text_y,
            font_size,
            self.text_color.to_rgba(),
            self.style.font_family.as_deref(),
        );

        renderer.pop_scissor();

        // draw dropdown arrow as a simple triangle using small rects
        let arrow_x = self.rect.x + self.rect.width - PADDING - ARROW_SIZE;
        let arrow_y = self.rect.y + (self.rect.height - ARROW_SIZE) / 2.0;
        let runic_gold = [217.0 / 255.0, 119.0 / 255.0, 6.0 / 255.0, 1.0];
        let arrow_color = runic_gold;

        if self.is_open {
            // up arrow when open: wide at bottom, narrow at top
            let row_height = ARROW_SIZE / 3.0;
            for row in 0..3 {
                let row_width = ARROW_SIZE - ((2 - row) as f32 * 2.0 * row_height / 1.5);
                let row_x = arrow_x + (ARROW_SIZE - row_width) / 2.0;
                let row_y = arrow_y + row as f32 * row_height;
                renderer.draw_rect(
                    Rect::new(row_x, row_y, row_width, row_height * 0.8),
                    arrow_color,
                    0.0,
                );
            }
        } else {
            // down arrow when closed: wide at top, narrow at bottom
            let row_height = ARROW_SIZE / 3.0;
            for row in 0..3 {
                let row_width = ARROW_SIZE - (row as f32 * 2.0 * row_height / 1.5);
                let row_x = arrow_x + (ARROW_SIZE - row_width) / 2.0;
                let row_y = arrow_y + row as f32 * row_height;
                renderer.draw_rect(
                    Rect::new(row_x, row_y, row_width, row_height * 0.8),
                    arrow_color,
                    0.0,
                );
            }
        }

        if self.is_open && !self.options.is_empty() {
            renderer.draw_overlay_rect(self.dropdown_rect, self.border_color.to_rgba(), 0.0);

            let dd_bg = self.dropdown_rect.shrink(border_width);
            renderer.draw_overlay_rect(dd_bg, self.dropdown_background.to_rgba(), 0.0);

            for (i, option) in self.options.iter().enumerate() {
                let item_rect = Rect::new(
                    dd_bg.x,
                    dd_bg.y + i as f32 * self.item_height,
                    dd_bg.width,
                    self.item_height,
                );

                if self.hovered_option == Some(i) {
                    renderer.draw_overlay_rect(item_rect, self.item_hover_color.to_rgba(), 0.0);
                }

                let item_text_y = item_rect.y + (self.item_height - font_size) / 2.0;
                renderer.draw_overlay_text(
                    option,
                    item_rect.x + PADDING,
                    item_text_y,
                    font_size,
                    self.text_color.to_rgba(),
                    self.style.font_family.as_deref(),
                );
            }
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

    /// Handle dropdown clicks with priority before other widgets
    fn overlay_update(&mut self, ctx: &mut UiContext) -> bool {
        if !self.is_open {
            return false;
        }

        let mouse_pos = ctx.mouse_pos();

        self.hovered_option = None;
        for (i, _) in self.options.iter().enumerate() {
            let item_rect = Rect::new(
                self.dropdown_rect.x,
                self.dropdown_rect.y + i as f32 * self.item_height,
                self.dropdown_rect.width,
                self.item_height,
            );
            if item_rect.contains(mouse_pos) {
                self.hovered_option = Some(i);
                break;
            }
        }

        if ctx.is_click_start() {
            if let Some(index) = self.hovered_option {
                self.selected_index = index;
                self.is_open = false;
                if let Some(id) = &self.style.id {
                    println!("[ComboBox] Selected index {}. ID: {}", index, id);
                }
                return true;
            }

            if self.dropdown_rect.contains(mouse_pos) {
                return true;
            }

            if self.rect.contains(mouse_pos) {
                self.is_open = false;
                if let Some(id) = &self.style.id {
                    println!("[ComboBox] Toggled. ID: {}, open: {}", id, self.is_open);
                }
                return true;
            }

            self.is_open = false;
            return false;
        }

        if self.dropdown_rect.contains(mouse_pos) {
            return true;
        }

        false
    }
}
