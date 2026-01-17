//! TabView widget - horizontal tabs with switchable content panels.
//!
//! Features:
//! - Row of tab labels at top
//! - Active tab has underline accent
//! - Only renders currently selected tab's content
//! - Click tabs to switch
//! - Find widgets in nested children

use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Color, Rect, Style};

use super::Widget;

/// Height of the tab bar area.
const TAB_BAR_HEIGHT: f32 = 32.0;
/// Font size for tab labels.
const TAB_FONT_SIZE: f32 = 14.0;
/// Underline thickness for active tab.
const UNDERLINE_HEIGHT: f32 = 2.0;
/// Spacing between tabs.
const TAB_SPACING: f32 = 24.0;
/// Padding inside each tab.
const TAB_PADDING: f32 = 8.0;
/// Diamond indicator size ratio (18.75% of tab bar height).
const DIAMOND_SIZE_RATIO: f32 = 0.1875;

pub struct TabView {
    pub style: Style,
    /// Tab labels.
    pub tabs: Vec<String>,
    /// Currently selected tab index.
    pub selected_index: usize,
    /// Content widgets for each tab.
    pub children: Vec<Box<dyn Widget>>,

    // Colors
    pub active_text_color: Color,
    pub inactive_text_color: Color,
    pub underline_color: Color,

    // Computed state
    rect: Rect,
    tab_bar_rect: Rect,
    content_rect: Rect,
    tab_rects: Vec<Rect>,
    hovered_tab: Option<usize>,
}

impl TabView {
    /// Creates a new TabView with the given tabs and style.
    pub fn new(tabs: Vec<String>, style: Style) -> Self {
        Self {
            style,
            tabs,
            selected_index: 0,
            children: Vec::new(),
            active_text_color: Color::Variable("highlight".to_string()),
            inactive_text_color: Color::Variable("text-dim".to_string()),
            underline_color: Color::Variable("runic-gold".to_string()),
            rect: Rect::default(),
            tab_bar_rect: Rect::default(),
            content_rect: Rect::default(),
            tab_rects: Vec::new(),
            hovered_tab: None,
        }
    }

    /// Builder: set selected tab index.
    pub fn with_selected_index(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }

    /// Builder: set active text color.
    pub fn with_active_text_color(mut self, color: Color) -> Self {
        self.active_text_color = color;
        self
    }

    /// Builder: set inactive text color.
    pub fn with_inactive_text_color(mut self, color: Color) -> Self {
        self.inactive_text_color = color;
        self
    }

    /// Builder: set underline color.
    pub fn with_underline_color(mut self, color: Color) -> Self {
        self.underline_color = color;
        self
    }

    /// Add a child widget (content panel for a tab).
    pub fn add_child(&mut self, child: Box<dyn Widget>) {
        self.children.push(child);
    }

    /// Get selected index.
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Set selected index.
    pub fn set_selected_index(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.selected_index = index;
        }
    }
}

impl Widget for TabView {
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
            .resolve_or(max_height, max_height)
            .min(max_height);

        self.rect = Rect::new(content_x, content_y, width, height);

        self.tab_bar_rect = Rect::new(self.rect.x, self.rect.y, self.rect.width, TAB_BAR_HEIGHT);

        self.content_rect = Rect::new(
            self.rect.x,
            self.rect.y + TAB_BAR_HEIGHT,
            self.rect.width,
            (self.rect.height - TAB_BAR_HEIGHT).max(0.0),
        );

        self.tab_rects.clear();

        // ! first pass: calculate total width of all tabs
        let mut total_tabs_width = 0.0;
        let mut tab_widths = Vec::new();
        for tab_label in &self.tabs {
            let char_width = TAB_FONT_SIZE * 0.6;
            let text_width = tab_label.len() as f32 * char_width;
            let tab_width = text_width + TAB_PADDING * 2.0;
            tab_widths.push(tab_width);
            total_tabs_width += tab_width;
        }
        if self.tabs.len() > 1 {
            total_tabs_width += TAB_SPACING * (self.tabs.len() - 1) as f32;
        }

        let start_x = self.tab_bar_rect.x + (self.tab_bar_rect.width - total_tabs_width) / 2.0;
        let mut x = start_x;
        for (i, tab_width) in tab_widths.iter().enumerate() {
            self.tab_rects.push(Rect::new(
                x,
                self.tab_bar_rect.y,
                *tab_width,
                TAB_BAR_HEIGHT,
            ));
            if i < tab_widths.len() - 1 {
                x += tab_width + TAB_SPACING;
            }
        }

        if let Some(child) = self.children.get_mut(self.selected_index) {
            child.layout(font_system, self.content_rect);
        }
    }

    fn update(&mut self, ctx: &mut UiContext) -> bool {
        let mouse_pos = ctx.mouse_pos();

        self.hovered_tab = None;
        for (i, tab_rect) in self.tab_rects.iter().enumerate() {
            if tab_rect.contains(mouse_pos) {
                self.hovered_tab = Some(i);
                break;
            }
        }

        if ctx.is_click_start() {
            if let Some(index) = self.hovered_tab {
                if index != self.selected_index {
                    self.selected_index = index;
                    if let Some(id) = &self.style.id {
                        println!("[TabView] Selected tab {}. ID: {}", index, id);
                    }
                    return true;
                }
            }
        }

        if let Some(child) = self.children.get_mut(self.selected_index) {
            return child.update(ctx);
        }

        false
    }

    fn overlay_update(&mut self, ctx: &mut UiContext) -> bool {
        if let Some(child) = self.children.get_mut(self.selected_index) {
            return child.overlay_update(ctx);
        }
        false
    }

    fn render(&self, renderer: &mut UiRenderer) {
        for (i, (tab_label, tab_rect)) in self.tabs.iter().zip(self.tab_rects.iter()).enumerate() {
            let is_active = i == self.selected_index;
            let is_hovered = self.hovered_tab == Some(i);

            let text_color = if is_active || is_hovered {
                self.active_text_color.to_rgba()
            } else {
                self.inactive_text_color.to_rgba()
            };

            let text_x = tab_rect.x + TAB_PADDING;
            let text_y = tab_rect.y + (tab_rect.height - TAB_FONT_SIZE) / 2.0;
            renderer.draw_text(
                tab_label,
                text_x,
                text_y,
                TAB_FONT_SIZE,
                text_color,
                self.style.font_family.as_deref(),
            );

            if is_active {
                let underline_y = tab_rect.y + tab_rect.height - UNDERLINE_HEIGHT;
                let char_width = TAB_FONT_SIZE * 0.6;
                let text_width = tab_label.len() as f32 * char_width;
                let underline_x = tab_rect.x + (tab_rect.width - text_width) / 2.0;
                let underline_rect =
                    Rect::new(underline_x, underline_y, text_width, UNDERLINE_HEIGHT);
                renderer.draw_rect(underline_rect, self.underline_color.to_rgba(), 0.0);
            }
        }

        if let Some(child) = self.children.get(self.selected_index) {
            child.render(renderer);
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

    fn for_each_child_mut(&mut self, f: &mut dyn FnMut(&mut dyn Widget)) {
        for child in &mut self.children {
            f(child.as_mut());
        }
    }

    fn find_widget_mut(&mut self, id: &str) -> Option<&mut dyn Widget> {
        if self.id() == Some(id) {
            return Some(self);
        }
        for child in &mut self.children {
            if let Some(found) = child.find_widget_mut(id) {
                return Some(found);
            }
        }
        None
    }
}
