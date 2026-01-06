use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Alignment, GridSpan, Rect, Style};

use super::Widget;

pub struct Column {
    pub style: Style,
    pub span: GridSpan,
    pub justify: Alignment,
    pub align: Alignment,
    pub children: Vec<Box<dyn Widget>>,
    rect: Rect,
}

impl Column {
    pub fn new(style: Style) -> Self {
        Self {
            style,
            span: GridSpan::default(),
            justify: Alignment::Start,
            align: Alignment::Start,
            children: Vec::new(),
            rect: Rect::default(),
        }
    }

    pub fn with_span(mut self, span: u8) -> Self {
        self.span = GridSpan::new(span);
        self
    }

    pub fn with_justify(mut self, justify: Alignment) -> Self {
        self.justify = justify;
        self
    }

    pub fn with_align(mut self, align: Alignment) -> Self {
        self.align = align;
        self
    }

    pub fn add_child(&mut self, child: Box<dyn Widget>) {
        self.children.push(child);
    }
}

impl Widget for Column {
    fn layout(&mut self, font_system: &mut FontSystem, available: Rect) {
        let (mt, mr, mb, ml) = self
            .style
            .resolve_margins(available.width, available.height);

        let content_x = available.x + ml;
        let content_y = available.y + mt;
        let max_width = available.width - ml - mr;
        let max_height = available.height - mt - mb;

        let width = self.style.width.resolve_or(max_width, max_width);
        let is_auto_height = self.style.height.resolve(max_height).is_none();

        // for height: Auto, layout children first to calculate natural height
        let mut child_heights = Vec::new();
        let height = if is_auto_height {
            if self.children.is_empty() {
                0.0
            } else {
                // estimate padding initially
                let (pt_est, pr_est, _pb_est, pl_est) =
                    self.style.resolve_padding(max_width, 100.0);
                let inner_width = max_width - pl_est - pr_est;

                // layout children with unlimited height to get their natural sizes
                let mut total_children_height = 0.0;
                let mut y_offset = 0.0;
                for child in &mut self.children {
                    let child_rect = Rect::new(
                        content_x + pl_est,
                        content_y + pt_est + y_offset,
                        inner_width,
                        999999.0,
                    );
                    child.layout(font_system, child_rect);
                    let child_height = child.rect().height;
                    child_heights.push(child_height);
                    total_children_height += child_height;
                    y_offset += child_height;
                }

                // recalculate padding with actual content height
                let (pt_final, _, pb_final, _) =
                    self.style.resolve_padding(max_width, total_children_height);
                total_children_height + pt_final + pb_final
            }
        } else {
            self.style.height.resolve_or(max_height, max_height)
        };

        self.rect = Rect::new(
            content_x,
            content_y,
            width.min(max_width),
            height.min(max_height),
        );

        let (pt, pr, pb, pl) = self
            .style
            .resolve_padding(self.rect.width, self.rect.height);
        let inner_rect = self.rect.shrink_by(pt, pr, pb, pl);

        if self.children.is_empty() {
            return;
        }

        let child_count = self.children.len();

        // for height: Auto, reposition children (they're already layouted with correct sizes)
        // for fixed height, distribute space according to justify
        if is_auto_height {
            let mut y_offset = 0.0;
            for (i, child) in self.children.iter_mut().enumerate() {
                let child_height = child_heights[i];
                let child_rect = Rect::new(
                    inner_rect.x,
                    inner_rect.y + y_offset,
                    inner_rect.width,
                    child_height,
                );
                child.layout(font_system, child_rect);
                y_offset += child_height;
            }
        } else {
            // fixed height: distribute space according to justify
            let total_height = inner_rect.height;

            let (gap, start_offset) = match self.justify {
                Alignment::Start => (0.0, 0.0),
                Alignment::Center => {
                    let child_height = total_height / child_count as f32;
                    (
                        0.0,
                        (total_height - child_height * child_count as f32) / 2.0,
                    )
                }
                Alignment::End => {
                    let child_height = total_height / child_count as f32;
                    (0.0, total_height - child_height * child_count as f32)
                }
                Alignment::SpaceBetween => {
                    if child_count > 1 {
                        let child_height = total_height / child_count as f32;
                        let gap = (total_height - child_height * child_count as f32)
                            / (child_count - 1) as f32;
                        (gap, 0.0)
                    } else {
                        (0.0, 0.0)
                    }
                }
                Alignment::SpaceAround => {
                    let child_height = total_height / child_count as f32;
                    let total_gap = total_height - child_height * child_count as f32;
                    let gap = total_gap / child_count as f32;
                    (gap, gap / 2.0)
                }
            };

            let mut y_offset = start_offset;
            for child in &mut self.children {
                let child_height = total_height / child_count as f32;
                let child_rect = Rect::new(
                    inner_rect.x,
                    inner_rect.y + y_offset,
                    inner_rect.width,
                    child_height,
                );
                child.layout(font_system, child_rect);
                y_offset += child_height + gap;
            }
        }
    }

    fn update(&mut self, ctx: &mut UiContext) -> bool {
        for child in &mut self.children {
            if child.update(ctx) {
                return true;
            }
        }

        if self.rect.contains(ctx.mouse_pos()) {
            if ctx.is_click_start() {
                if let Some(id) = &self.style.id {
                    println!("[Column] Clicked. ID: {}", id);
                }
                return true;
            }
        }
        false
    }

    fn render(&self, renderer: &mut UiRenderer) {
        let border_rgba = self.style.border_color.to_rgba();
        let has_border = self.style.border_width > 0.0 && border_rgba[3] > 0.0;
        let border_radius = self.style.border_radius;
        let bg_color = self.style.background.to_rgba();
        let has_opaque_bg = bg_color[3] > 0.0;

        if has_border && has_opaque_bg {
            // opaque background: draw border as full rect, then background on top (supports rounded corners)
            renderer.draw_rect(self.rect, border_rgba, border_radius);

            let bg_rect = self.rect.shrink(self.style.border_width);
            let inner_radius = (border_radius - self.style.border_width).max(0.0);
            if bg_rect.width > 0.0 && bg_rect.height > 0.0 {
                renderer.draw_rect(bg_rect, bg_color, inner_radius);
            }
        } else if has_border {
            // transparent background: draw border as 4 rectangles (outline only)
            let bw = self.style.border_width;
            // top
            renderer.draw_rect(
                Rect::new(self.rect.x, self.rect.y, self.rect.width, bw),
                border_rgba,
                0.0,
            );
            // bottom
            renderer.draw_rect(
                Rect::new(
                    self.rect.x,
                    self.rect.y + self.rect.height - bw,
                    self.rect.width,
                    bw,
                ),
                border_rgba,
                0.0,
            );
            // left
            renderer.draw_rect(
                Rect::new(
                    self.rect.x,
                    self.rect.y + bw,
                    bw,
                    self.rect.height - 2.0 * bw,
                ),
                border_rgba,
                0.0,
            );
            // right
            renderer.draw_rect(
                Rect::new(
                    self.rect.x + self.rect.width - bw,
                    self.rect.y + bw,
                    bw,
                    self.rect.height - 2.0 * bw,
                ),
                border_rgba,
                0.0,
            );
        } else if has_opaque_bg {
            // no border, just background
            renderer.draw_rect(self.rect, bg_color, border_radius);
        }

        for child in &self.children {
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
            f(&mut **child);
        }
    }

    fn find_widget_mut(&mut self, id: &str) -> Option<&mut dyn Widget> {
        if self.id() == Some(id) {
            return Some(self);
        }
        for child in &mut self.children {
            if let Some(w) = child.find_widget_mut(id) {
                return Some(w);
            }
        }
        None
    }

    fn grid_span(&self) -> Option<GridSpan> {
        Some(self.span)
    }
}
