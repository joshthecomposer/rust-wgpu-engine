//! ScrollView widget with viewport/content clipping using glScissor.
//!
//! This widget provides a scrollable container that clips children to a viewport area.
//! Supports both scrollbar dragging and mouse wheel scrolling.

use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Alignment, Rect, ScrollbarStyle, Style};

use super::Widget;

/// A scrollable container widget that clips children to a viewport.
///
/// Children are laid out vertically and distributed evenly across `content_height`.
/// When `content_height` exceeds the viewport height, a scrollbar appears and
/// content can be scrolled via dragging the thumb, clicking the track, or using
/// the mouse wheel.
///
/// # Layout Behavior
///
/// Unlike other widgets, ScrollView uses its explicit `height` style property
/// directly, rather than clamping to the parent's available space. This allows
/// it to maintain a fixed viewport size regardless of parent constraints.
///
/// # Scrollbar Styling
///
/// The scrollbar appearance can be customized via `scrollbar_style`:
/// ```ron
/// ScrollView(
///     scrollbar_style: (
///         width: 12.0,
///         thumb_color: Rgba(0.5, 0.5, 0.5, 1.0),
///         thumb_hover_color: Variable("accent"),
///     ),
///     // ...
/// )
/// ```
pub struct ScrollView {
    pub style: Style,
    /// Total height of the scrollable content area.
    pub content_height: f32,
    pub justify: Alignment,
    pub children: Vec<Box<dyn Widget>>,
    /// Configurable scrollbar appearance.
    pub scrollbar_style: ScrollbarStyle,

    /// Computed bounding rect after layout.
    rect: Rect,
    /// Current vertical scroll offset in pixels.
    scroll_offset: f32,

    // Scrollbar interaction state
    is_dragging: bool,
    drag_start_y: f32,
    drag_start_offset: f32,
    is_hovered: bool,
}

impl ScrollView {
    /// Creates a new ScrollView with the given style and content height.
    pub fn new(style: Style, content_height: f32) -> Self {
        Self {
            style,
            content_height,
            justify: Alignment::Start,
            children: Vec::new(),
            scrollbar_style: ScrollbarStyle::default(),
            rect: Rect::default(),
            scroll_offset: 0.0,
            is_dragging: false,
            drag_start_y: 0.0,
            drag_start_offset: 0.0,
            is_hovered: false,
        }
    }

    /// Sets the content justification alignment.
    pub fn with_justify(mut self, justify: Alignment) -> Self {
        self.justify = justify;
        self
    }

    /// Sets custom scrollbar styling.
    pub fn with_scrollbar_style(mut self, scrollbar_style: ScrollbarStyle) -> Self {
        self.scrollbar_style = scrollbar_style;
        self
    }

    /// Adds a child widget to this ScrollView.
    pub fn add_child(&mut self, child: Box<dyn Widget>) {
        self.children.push(child);
    }

    /// Returns the viewport rect (visible content area, excluding scrollbar).
    fn viewport_rect(&self) -> Rect {
        let scrollbar_visible = self.content_height > self.rect.height;
        let width_reduction = if scrollbar_visible {
            self.scrollbar_style.width()
        } else {
            0.0
        };
        Rect::new(
            self.rect.x,
            self.rect.y,
            self.rect.width - width_reduction,
            self.rect.height,
        )
    }

    /// Returns the scrollbar track rect (full vertical area for the scrollbar).
    fn scrollbar_track_rect(&self) -> Rect {
        let width = self.scrollbar_style.width();
        Rect::new(
            self.rect.x + self.rect.width - width,
            self.rect.y,
            width,
            self.rect.height,
        )
    }

    /// Returns the scrollbar thumb rect based on current scroll position.
    ///
    /// The thumb size is proportional to the viewport/content ratio,
    /// and its position reflects the current scroll offset.
    fn scrollbar_thumb_rect(&self) -> Rect {
        let width = self.scrollbar_style.width();
        let min_height = self.scrollbar_style.thumb_min_height();
        let viewport_height = self.rect.height;
        let max_scroll = (self.content_height - viewport_height).max(0.0);
        let scroll_ratio = if max_scroll > 0.0 {
            self.scroll_offset / max_scroll
        } else {
            0.0
        };

        let thumb_height = (viewport_height * (viewport_height / self.content_height))
            .max(min_height)
            .min(viewport_height);
        let available_track = viewport_height - thumb_height;
        let thumb_y = self.rect.y + scroll_ratio * available_track;

        Rect::new(
            self.rect.x + self.rect.width - width,
            thumb_y,
            width,
            thumb_height,
        )
    }

    /// Clamps `scroll_offset` to the valid range `[0, max_scroll]`.
    fn clamp_scroll(&mut self) {
        let max_scroll = (self.content_height - self.rect.height).max(0.0);
        self.scroll_offset = self.scroll_offset.clamp(0.0, max_scroll);
    }
}

impl Widget for ScrollView {
    /// Computes layout for this ScrollView and all children.
    ///
    /// Width is clamped to available space, but height uses the explicit style
    /// value directly (allowing ScrollView to overflow its parent if needed).
    /// Children are laid out vertically with heights distributed evenly across
    /// `content_height`, offset by the current `scroll_offset`.
    fn layout(&mut self, font_system: &mut FontSystem, available: Rect) {
        let (mt, mr, mb, ml) = self
            .style
            .resolve_margins(available.width, available.height);

        let content_x = available.x + ml;
        let content_y = available.y + mt;
        let max_width = available.width - ml - mr;

        // Width: clamp to available space
        let width = self
            .style
            .width
            .resolve_or(max_width, max_width)
            .min(max_width);

        // Height: use explicit value directly (don't clamp to available)
        let height = self
            .style
            .height
            .resolve_or(available.height - mt - mb, available.height - mt - mb);

        self.rect = Rect::new(content_x, content_y, width, height);
        self.clamp_scroll();

        let (pt, pr, pb, pl) = self
            .style
            .resolve_padding(self.rect.width, self.rect.height);

        let viewport = self.viewport_rect();
        let inner_rect = viewport.shrink_by(pt, pr, pb, pl);

        if self.children.is_empty() {
            return;
        }

        let child_count = self.children.len();
        let child_height = self.content_height / child_count as f32;

        for (i, child) in self.children.iter_mut().enumerate() {
            let child_y = inner_rect.y + (i as f32 * child_height) - self.scroll_offset;
            let child_rect = Rect::new(inner_rect.x, child_y, inner_rect.width, child_height);
            child.layout(font_system, child_rect);
        }
    }

    /// Handles input events for scrolling and child interaction.
    ///
    /// Supports:
    /// - Scrollbar thumb dragging
    /// - Click-to-jump on scrollbar track
    /// - Mouse wheel scrolling (40px per line)
    /// - Forwarding events to children within viewport
    fn update(&mut self, ctx: &mut UiContext) -> bool {
        let mouse_pos = ctx.mouse_pos();
        let viewport = self.viewport_rect();
        let min_height = self.scrollbar_style.thumb_min_height();

        // Handle active scrollbar drag
        if self.is_dragging {
            if ctx.is_mouse_down() {
                let delta_y = mouse_pos.y - self.drag_start_y;
                let viewport_height = self.rect.height;
                let max_scroll = (self.content_height - viewport_height).max(0.0);

                let thumb_height = (viewport_height * (viewport_height / self.content_height))
                    .max(min_height)
                    .min(viewport_height);
                let available_track = viewport_height - thumb_height;

                if available_track > 0.0 {
                    let scroll_delta = (delta_y / available_track) * max_scroll;
                    self.scroll_offset = self.drag_start_offset + scroll_delta;
                    self.clamp_scroll();
                }
                return true;
            } else {
                self.is_dragging = false;
            }
        }

        // Scrollbar interaction
        let thumb_rect = self.scrollbar_thumb_rect();
        let track_rect = self.scrollbar_track_rect();

        self.is_hovered = thumb_rect.contains(mouse_pos);

        if ctx.is_click_start() {
            if thumb_rect.contains(mouse_pos) {
                self.is_dragging = true;
                self.drag_start_y = mouse_pos.y;
                self.drag_start_offset = self.scroll_offset;
                return true;
            } else if track_rect.contains(mouse_pos) {
                // Jump to clicked position on track
                let relative_y = mouse_pos.y - self.rect.y;
                let scroll_ratio = relative_y / self.rect.height;
                let max_scroll = (self.content_height - self.rect.height).max(0.0);
                self.scroll_offset = scroll_ratio * max_scroll;
                self.clamp_scroll();
                return true;
            }
        }

        // Mouse wheel and child updates (only when mouse is in viewport)
        if viewport.contains(mouse_pos) {
            let scroll_wheel = ctx.scroll_delta();
            if scroll_wheel.y != 0.0 {
                const SCROLL_SPEED: f32 = 40.0;
                self.scroll_offset -= scroll_wheel.y * SCROLL_SPEED;
                self.clamp_scroll();
                return true;
            }

            for child in &mut self.children {
                if child.update(ctx) {
                    return true;
                }
            }
        }

        if self.rect.contains(mouse_pos) && ctx.is_click_start() {
            if let Some(id) = &self.style.id {
                println!("[ScrollView] Clicked. ID: {}", id);
            }
            return true;
        }

        false
    }

    /// Renders the ScrollView background, scrollbar, and clipped children.
    ///
    /// Uses glScissor to clip children to the viewport area.
    fn render(&self, renderer: &mut UiRenderer) {
        let bg_color = self.style.background.to_rgba();
        if bg_color[3] > 0.0 {
            renderer.draw_rect(self.rect, bg_color);
        }

        let scrollbar_visible = self.content_height > self.rect.height;

        // Draw track behind content
        if scrollbar_visible {
            let track_rect = self.scrollbar_track_rect();
            renderer.draw_rect(track_rect, self.scrollbar_style.track_color());
        }

        // Clip children to viewport
        let viewport = self.viewport_rect();
        renderer.push_scissor(viewport);

        for child in &self.children {
            child.render(renderer);
        }

        renderer.pop_scissor();

        // Draw thumb on top
        if scrollbar_visible {
            let thumb_rect = self.scrollbar_thumb_rect();
            let thumb_color = if self.is_dragging {
                self.scrollbar_style.thumb_active_color()
            } else if self.is_hovered {
                self.scrollbar_style.thumb_hover_color()
            } else {
                self.scrollbar_style.thumb_color()
            };
            renderer.draw_rect(thumb_rect, thumb_color);
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }
}
