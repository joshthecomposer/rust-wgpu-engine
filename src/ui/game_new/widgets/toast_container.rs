//! ToastContainer widget - manages dynamic toast notifications.
//!
//! Features:
//! - Stacks toasts vertically from bottom-right, upward
//! - Max 5 toasts displayed simultaneously
//! - Three animation states: Entering (slide-in 0-300ms), Visible, Exiting (fade-out 200ms)
//! - Handles close button events via UIContext message queue
//! - Auto-removes expired toasts and marks layout dirty

use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Alignment, Color, Length, Rect, Style};
use crate::ui::game_new::widgets::{
    BoxWidget, CloseButton, Column, Label, Row, TextureRect, Widget,
};
use crate::ui::toast::ToastType;

/// Animation state for a toast notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToastState {
    Entering, // slide in from right (0-300ms)
    Visible,  // fully shown (300ms - duration)
    Exiting,  // fade out (duration - duration+200ms)
}

/// Internal data for an active toast notification.
struct ToastData {
    id: u32,
    toast_type: ToastType,
    title: String, // for debugging
    created_at: f64,
    duration: f64,
    state: ToastState,
    widget: Box<dyn Widget>, // the composed Row widget
    slide_offset: f32,       // current X offset for slide animation
    alpha: f32,              // current alpha for fade animation
}

/// Container widget that manages dynamic toast notifications.
pub struct ToastContainer {
    pub style: Style,
    toasts: Vec<ToastData>,
    next_id: u32,
    rect: Rect,
    max_toasts: usize,
    enter_duration: f64,
    exit_duration: f64,
    gap: f32, // vertical spacing between toasts
    pub needs_layout_update: bool,
}

impl ToastContainer {
    /// Create a new ToastContainer with default settings.
    pub fn new(style: Style) -> Self {
        Self {
            style,
            toasts: Vec::new(),
            next_id: 0,
            rect: Rect::default(),
            max_toasts: 5,
            enter_duration: 0.3,
            exit_duration: 0.3, // match Slint animation duration (300ms)
            gap: 8.0,
            needs_layout_update: false,
        }
    }

    /// Get the number of active toasts (for debug).
    pub fn toast_count(&self) -> usize {
        self.toasts.len()
    }

    /// Add a new toast notification.
    pub fn add_toast(
        &mut self,
        toast_type: ToastType,
        title: String,
        message: String,
        duration: Option<f64>,
        created_at: f64,
        icon_texture: Option<u32>,
    ) {
        // enforce max toasts limit - remove oldest if at limit
        if self.toasts.len() >= self.max_toasts {
            self.toasts.remove(0);
            self.needs_layout_update = true;
        }

        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        // build toast widget: Row containing icon, labels, close button
        let widget = self.build_toast_widget(id, toast_type, title.clone(), message, icon_texture);

        self.toasts.push(ToastData {
            id,
            toast_type,
            title,
            created_at,
            duration: duration.unwrap_or(5.0),
            state: ToastState::Entering,
            widget,
            slide_offset: 400.0, // start off-screen to the right (full toast width)
            alpha: 1.0,
        });

        self.needs_layout_update = true;
    }

    /// Build the widget tree for a single toast.
    fn build_toast_widget(
        &self,
        id: u32,
        toast_type: ToastType,
        title: String,
        message: String,
        icon_texture: Option<u32>,
    ) -> Box<dyn Widget> {
        // println!(
        //     "[ToastContainer::build_toast_widget] building toast id={}",
        //     id
        // );
        // Determine accent color per toast type (for left accent bar)
        let accent_color = match toast_type {
            ToastType::Success => Color::Hex("#d97706".to_string()), // runic-gold
            ToastType::Error => Color::Hex("#991b1b".to_string()),   // blood-hp
            ToastType::Warning => Color::Hex("#d97706".to_string()), // icon-glow
            ToastType::Info => Color::Hex("#075985".to_string()),    // mana-sky
        };

        // Main toast background style (gradient approximated with stone-dark)
        let mut toast_row_style = Style::default();
        toast_row_style.id = Some(format!("toast_{}", id));
        toast_row_style.width = Length::Px(370.0); // match container width from toast_container.ron
        toast_row_style.height = Length::Px(80.0); // fixed height approximation (enough for content)
        toast_row_style.padding = Length::Px(0.0); // no padding on outer row, inner row will have padding
                                                   // Background gradient: linear-gradient(90deg, Theme.stone-dark 0%, Theme.deep-void 100%)
                                                   // We'll approximate with stone-dark for now; deep-void may be a darker color.
        toast_row_style.background = Color::Hex("#1e293b".to_string()); // stone-dark
        toast_row_style.border_radius = 4.0;
        toast_row_style.border_width = 2.0;
        toast_row_style.border_color = Color::Hex("#475569".to_string()); // stone-light

        // Create a Row that will contain accent bar and content row
        let mut toast_row = Row::new(toast_row_style);
        toast_row.align = Alignment::Start;

        // Left accent bar (4px wide, full height)
        let mut accent_style = Style::default();
        accent_style.width = Length::Px(4.0);
        accent_style.height = Length::Percent(100.0);
        accent_style.background = accent_color.clone();
        let accent_bar = BoxWidget::new(accent_style);
        toast_row.add_child(Box::new(accent_bar));

        // Inner content row (icon, text, close button) with proper spacing
        let mut content_row_style = Style::default();
        content_row_style.width = Length::Auto;
        content_row_style.height = Length::Auto;
        // Padding inside the content row (left 12px, right 16px, top/bottom 12px)
        content_row_style.padding = Length::Px(12.0); // top/bottom
        content_row_style.padding_left = Some(Length::Px(12.0));
        content_row_style.padding_right = Some(Length::Px(16.0));
        let mut content_row = Row::new(content_row_style);
        content_row.align = Alignment::Start;
        // Row spacing is not supported; we'll use margins between children.

        // icon - simple 'i' without circle
        // User requested: "replace the icon with just an 'i' - no circle"
        let mut icon_style = Style::default();
        icon_style.width = Length::Px(32.0); // full icon width
        icon_style.height = Length::Px(32.0);
        icon_style.margin_right = Some(Length::Px(6.0)); // reduced margin
        icon_style.font_size = Some(24.0); // larger font for visibility
        icon_style.color = Some(accent_color.clone()); // accent color text
        icon_style.margin_top = Some(Length::Px(4.0));
        icon_style.text_align = Some(Alignment::Center);

        let icon_label = Label::new("i".to_string(), icon_style);
        content_row.add_child(Box::new(icon_label));

        // content column (title + message)
        let mut content_col_style = Style::default();
        content_col_style.width = Length::Auto;
        content_col_style.height = Length::Auto;
        content_col_style.margin_right = Some(Length::Px(10.0));

        let mut content_col = Column::new(content_col_style);
        content_col.justify = Alignment::Start;

        // title label
        let mut title_style = Style::default();
        title_style.font_size = Some(14.0);
        // highlight: #fef3c7
        title_style.color = Some(Color::Hex("#fef3c7".to_string()));
        // Increased bottom margin to prevent overlap with message
        title_style.margin_bottom = Some(Length::Px(6.0));

        let title_label = Label::new(title, title_style);
        content_col.add_child(Box::new(title_label));

        // message label
        // Helper to wrap text approx at N chars
        // Reduced to 25 chars to ensure it fits within the column (approx 280px width)
        let max_chars = 25;
        let wrapped_message = if message.len() > max_chars {
            let mut result = String::new();
            let mut current_line_len = 0;
            for word in message.split_whitespace() {
                if current_line_len + word.len() > max_chars {
                    result.push('\n');
                    result.push_str(word);
                    current_line_len = word.len();
                } else {
                    if current_line_len > 0 {
                        result.push(' ');
                        current_line_len += 1;
                    }
                    result.push_str(word);
                    current_line_len += word.len();
                }
            }
            result
        } else {
            message
        };

        let mut message_style = Style::default();
        message_style.font_size = Some(13.0);
        // old-text: #94a3b8
        message_style.color = Some(Color::Hex("#94a3b8".to_string()));

        let message_label = Label::new(wrapped_message, message_style);
        content_col.add_child(Box::new(message_label));

        content_row.add_child(Box::new(content_col));

        // close button with custom colors: normal stone-light, hover old-text
        let mut close_style = Style::default();
        close_style.id = Some(format!("toast_close_{}", id));
        close_style.width = Length::Px(20.0);
        close_style.height = Length::Px(20.0);
        // no margin needed because content column already has margin_right

        let close_btn = CloseButton::new(close_style)
            .with_normal_color(Color::Hex("#475569".to_string())) // stone-light
            .with_hover_color(Color::Hex("#94a3b8".to_string())); // old-text
        content_row.add_child(Box::new(close_btn));

        // Add the content row to the outer toast row (after accent bar)
        toast_row.add_child(Box::new(content_row));

        // Create a wrapper Column that stacks shadow behind body
        let mut wrapper_style = Style::default();
        wrapper_style.width = Length::Px(370.0);
        wrapper_style.height = Length::Px(84.0); // fixed height: toast height + shadow offset
        wrapper_style.padding = Length::Px(0.0);
        wrapper_style.background = Color::transparent();
        let mut wrapper_col = Column::new(wrapper_style);
        wrapper_col.justify = Alignment::Start;
        wrapper_col.align = Alignment::Start;

        // Shadow rectangle (offset 4px right and down)
        let mut shadow_style = Style::default();
        shadow_style.width = Length::Px(370.0);
        shadow_style.height = Length::Px(80.0); // same as toast height
        shadow_style.background = Color::Rgba(0.0, 0.0, 0.0, 0.5); // #000000.with-alpha(0.5)
        shadow_style.margin_left = Some(Length::Px(4.0));
        shadow_style.margin_top = Some(Length::Px(4.0));
        let shadow_box = BoxWidget::new(shadow_style);
        wrapper_col.add_child(Box::new(shadow_box));

        // Toast body (overlap shadow by moving up 4px)
        let mut body_style = Style::default();
        body_style.width = Length::Px(370.0);
        body_style.height = Length::Px(80.0);
        body_style.margin_top = Some(Length::Px(-84.0)); // overlap shadow completely (80px height + 4px offset)
        body_style.margin_left = Some(Length::Px(0.0));
        let mut body_col = Column::new(body_style);
        body_col.justify = Alignment::Start;
        body_col.align = Alignment::Start;
        body_col.add_child(Box::new(toast_row));
        wrapper_col.add_child(Box::new(body_col));

        Box::new(wrapper_col)
    }

    /// Update toast states based on elapsed time.
    fn update_toast_states(&mut self, elapsed_time: f64) {
        let mut state_changed = false;

        for toast in &mut self.toasts {
            let age = elapsed_time - toast.created_at;

            match toast.state {
                ToastState::Entering => {
                    // interpolate slide offset (toast width -> 0 over 0.3s)
                    let progress = (age / self.enter_duration).min(1.0) as f32;
                    toast.slide_offset = 400.0 * (1.0 - progress);

                    if age >= self.enter_duration {
                        toast.state = ToastState::Visible;
                        toast.slide_offset = 0.0;
                        state_changed = true;
                    }
                }
                ToastState::Visible => {
                    if age >= toast.duration {
                        toast.state = ToastState::Exiting;
                        state_changed = true;
                    }
                }
                ToastState::Exiting => {
                    // interpolate alpha (1.0 -> 0.0 over 0.2s)
                    let exit_age = age - toast.duration;
                    let progress = (exit_age / self.exit_duration).min(1.0) as f32;
                    toast.alpha = 1.0 - progress;
                }
            }
        }

        // remove toasts that have finished exiting
        let old_len = self.toasts.len();
        self.toasts.retain(|toast| {
            let age = elapsed_time - toast.created_at;
            if toast.state == ToastState::Exiting {
                age < toast.duration + self.exit_duration
            } else {
                true
            }
        });

        if self.toasts.len() != old_len {
            state_changed = true;
        }

        if state_changed {
            self.needs_layout_update = true;
        }
    }

    /// Check for close button events and remove corresponding toasts.
    fn handle_close_events(&mut self, _ctx: &mut UiContext) {
        // This method is currently a stub; close events are handled in update().
        // No action needed.
    }

    /// Manually remove a toast by ID (e.g., when close button is clicked).
    pub fn remove_toast(&mut self, id: u32) {
        if let Some(pos) = self.toasts.iter().position(|t| t.id == id) {
            self.toasts.remove(pos);
            self.needs_layout_update = true;
        }
    }

    /// Update animation states for all toasts based on elapsed time.
    /// This should be called every frame to advance animations.
    pub fn update_states(&mut self, elapsed_time: f64) {
        self.update_toast_states(elapsed_time);
    }
}

impl Widget for ToastContainer {
    fn layout(&mut self, font_system: &mut FontSystem, available: Rect) {
        let (mt, mr, mb, ml) = self
            .style
            .resolve_margins(available.width, available.height);

        let content_x = available.x + ml;
        let content_y = available.y + mt;
        let max_width = available.width - ml - mr;
        let max_height = available.height - mt - mb;

        let width = self.style.width.resolve_or(max_width, max_width);

        // For height, if Auto, calculate based on toast content
        let height = if matches!(self.style.height, Length::Auto) {
            // Calculate total height needed for toasts
            let (pt, pr, pb, pl) = self.style.resolve_padding(width, max_height);
            let mut total_height = pt + pb;

            // Temporarily layout toasts to get their heights
            for toast in self.toasts.iter_mut() {
                let temp_rect = Rect::new(0.0, 0.0, width - pl - pr, 9999.0);
                toast.widget.layout(font_system, temp_rect);
                total_height += toast.widget.rect().height;
                if total_height > pt + pb {
                    total_height += self.gap; // gap between toasts
                }
            }

            // Apply max_height constraint
            let max_h = self.style.max_height.resolve_or(max_height, max_height);
            total_height.min(max_h).min(max_height)
        } else {
            self.style.height.resolve_or(max_height, max_height)
        };

        self.rect = Rect::new(
            content_x,
            content_y,
            width.min(max_width),
            height.min(max_height),
        );

        // println!("[ToastContainer] Computed rect: {:?}", self.rect);
        // println!(
        //     "[ToastContainer] Number of toasts to layout: {}",
        //     self.toasts.len()
        // );

        // layout toasts from bottom-up
        let (pt, pr, pb, pl) = self
            .style
            .resolve_padding(self.rect.width, self.rect.height);
        let inner_rect = self.rect.shrink_by(pt, pr, pb, pl);

        // calculate total height needed and layout toasts
        let mut y_offset = inner_rect.height; // start at bottom

        // layout in reverse order (newest at bottom)
        for toast in self.toasts.iter_mut().rev() {
            // layout toast to get its natural height
            let toast_rect = Rect::new(inner_rect.x, inner_rect.y, inner_rect.width, 9999.0);
            toast.widget.layout(font_system, toast_rect);

            let toast_height = toast.widget.rect().height;
            // println!(
            //     "[ToastContainer] Toast widget height: {}, rect: {:?}",
            //     toast_height,
            //     toast.widget.rect()
            // );
            y_offset -= toast_height;

            // apply slide offset (shift X position during animation)
            let final_x = inner_rect.x + toast.slide_offset;
            let final_y = inner_rect.y + y_offset;

            let final_rect = Rect::new(final_x, final_y, inner_rect.width, toast_height);
            // println!(
            //     "[ToastContainer] Laying out toast at final_rect: {:?}",
            //     final_rect
            // );
            toast.widget.layout(font_system, final_rect);

            y_offset -= self.gap; // add gap for next toast
        }

        self.needs_layout_update = false;
    }

    fn update(&mut self, ctx: &mut UiContext) -> bool {
        // process children in REVERSE order (bottom-to-top, newest first)
        for toast in self.toasts.iter_mut().rev() {
            if toast.widget.update(ctx) {
                // check if close button was clicked
                if let Some(close_btn_id) = toast
                    .widget
                    .find_widget_mut(&format!("toast_close_{}", toast.id))
                {
                    // transition to exiting state
                    toast.state = ToastState::Exiting;
                    self.needs_layout_update = true;
                }
                return true;
            }
        }

        false
    }

    fn render(&self, renderer: &mut UiRenderer) {
        // render toasts in order (oldest first, so newest is on top visually)
        for toast in &self.toasts {
            // modulate alpha for fade-out effect
            if toast.state == ToastState::Exiting && toast.alpha < 1.0 {
                // TODO: apply alpha to renderer (might need renderer support)
                // for now, just render normally
                toast.widget.render(renderer);
            } else {
                toast.widget.render(renderer);
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

    fn for_each_child_mut(&mut self, f: &mut dyn FnMut(&mut dyn Widget)) {
        for toast in &mut self.toasts {
            f(&mut *toast.widget);
        }
    }

    fn find_widget_mut(&mut self, id: &str) -> Option<&mut dyn Widget> {
        if self.id() == Some(id) {
            return Some(self);
        }
        for toast in &mut self.toasts {
            if let Some(w) = toast.widget.find_widget_mut(id) {
                return Some(w);
            }
        }
        None
    }
}
