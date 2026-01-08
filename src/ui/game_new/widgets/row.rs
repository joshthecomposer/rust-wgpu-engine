use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Alignment, GridSpan, Rect, Style};

use super::Widget;

pub struct Row {
    pub style: Style,
    pub justify: Alignment,
    pub align: Alignment,
    pub children: Vec<Box<dyn Widget>>,
    rect: Rect,
}

impl Row {
    pub fn new(style: Style) -> Self {
        Self {
            style,
            justify: Alignment::Start,
            align: Alignment::Start,
            children: Vec::new(),
            rect: Rect::default(),
        }
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

impl Widget for Row {
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

        // For height: Auto, calculate from children's maximum height
        let height = if is_auto_height {
            if self.children.is_empty() {
                0.0
            } else {
                // initially estimated padding
                let (pt_est, pr_est, pb_est, pl_est) = self.style.resolve_padding(max_width, 100.0);
                let inner_width = max_width - pl_est - pr_est;
                let _inner_height = max_height - pt_est - pb_est;

                // layout children to get their natural heights
                // guesstimate with a reasonable constraint (not unlimited, not full screen)
                // children with Auto height should calculate from their content
                let mut max_child_height: f32 = 0.0;
                let child_count = self.children.len();
                let total_width = inner_width;
                let child_width = total_width / child_count as f32;

                // reasonable max height constraint (e.g., 200px for HUD elements)
                let reasonable_max_height = 200.0;

                for child in &mut self.children {
                    let child_rect = Rect::new(
                        content_x + pl_est,
                        content_y + pt_est,
                        child_width,
                        reasonable_max_height,
                    );
                    child.layout(font_system, child_rect);
                    let child_used_height = child.rect().height;
                    max_child_height = max_child_height.max(child_used_height);
                }

                // recalculate padding with actual content height
                let (pt_final, _, pb_final, _) =
                    self.style.resolve_padding(max_width, max_child_height);
                max_child_height + pt_final + pb_final
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
        let total_width = inner_rect.width;

        // for height: Auto, children are already layouted, just reposition them horizontally
        // for fixed height, use normal layout
        if is_auto_height {
            // reposition children horizontally (they're already layouted with correct sizes)
            // check if all children are Columns with spans (grid layout)
            let spans: Vec<Option<GridSpan>> =
                self.children.iter().map(|c| c.grid_span()).collect();
            let all_columns = spans.iter().all(|s| s.is_some());

            if all_columns {
                // grid layout: use span values
                let total_span: u8 = spans.iter().map(|s| s.unwrap().0).sum();
                let span_unit_width = if total_span > 0 {
                    total_width / total_span as f32
                } else {
                    total_width / child_count as f32
                };

                let mut x_offset = 0.0;
                for (i, child) in self.children.iter_mut().enumerate() {
                    let span = spans[i].unwrap().0;
                    let child_width = span as f32 * span_unit_width;
                    let child_height = child.rect().height;
                    let child_rect = Rect::new(
                        inner_rect.x + x_offset,
                        inner_rect.y,
                        child_width,
                        child_height,
                    );
                    child.layout(font_system, child_rect);
                    x_offset += child_width;
                }
            } else {
                // Layout children with unlimited width first to get their natural widths
                let mut child_widths = Vec::new();
                let mut total_fixed_width = 0.0;
                let mut flexible_count = 0;

                for child in &mut self.children {
                    let temp_rect = Rect::new(
                        inner_rect.x,
                        inner_rect.y,
                        999999.0, // unlimited width to get natural size
                        inner_rect.height,
                    );
                    child.layout(font_system, temp_rect);
                    let natural_width = child.rect().width;
                    child_widths.push(natural_width);
                    if natural_width < 999999.0 {
                        total_fixed_width += natural_width;
                    } else {
                        flexible_count += 1;
                    }
                }

                // distribute remaining space to flexible children
                let remaining_width = (total_width - total_fixed_width).max(0.0);
                let flexible_width = if flexible_count > 0 {
                    remaining_width / flexible_count as f32
                } else {
                    0.0
                };

                // position children with their calculated widths
                let mut x_offset = 0.0;
                for (i, child) in self.children.iter_mut().enumerate() {
                    let child_width = if child_widths[i] < 999999.0 {
                        child_widths[i]
                    } else {
                        flexible_width
                    };
                    let child_height = child.rect().height;
                    let child_rect = Rect::new(
                        inner_rect.x + x_offset,
                        inner_rect.y,
                        child_width,
                        child_height,
                    );
                    child.layout(font_system, child_rect);
                    x_offset += child_width;
                }
            }
            return;
        }

        // check if all children are Columns with spans (grid layout)
        let spans: Vec<Option<GridSpan>> = self.children.iter().map(|c| c.grid_span()).collect();
        let all_columns = spans.iter().all(|s| s.is_some());

        if all_columns {
            // grid layout: use span values
            let total_span: u8 = spans.iter().map(|s| s.unwrap().0).sum();
            let span_unit_width = if total_span > 0 {
                total_width / total_span as f32
            } else {
                total_width / child_count as f32
            };

            let (gap, start_offset) = match self.justify {
                Alignment::Start => (0.0, 0.0),
                Alignment::Center => {
                    let used_width: f32 = spans
                        .iter()
                        .map(|s| s.unwrap().0 as f32 * span_unit_width)
                        .sum();
                    (0.0, (total_width - used_width) / 2.0)
                }
                Alignment::End => {
                    let used_width: f32 = spans
                        .iter()
                        .map(|s| s.unwrap().0 as f32 * span_unit_width)
                        .sum();
                    (0.0, total_width - used_width)
                }
                Alignment::SpaceBetween => {
                    if child_count > 1 {
                        let used_width: f32 = spans
                            .iter()
                            .map(|s| s.unwrap().0 as f32 * span_unit_width)
                            .sum();
                        let gap = (total_width - used_width) / (child_count - 1) as f32;
                        (gap, 0.0)
                    } else {
                        (0.0, 0.0)
                    }
                }
                Alignment::SpaceAround => {
                    let used_width: f32 = spans
                        .iter()
                        .map(|s| s.unwrap().0 as f32 * span_unit_width)
                        .sum();
                    let total_gap = total_width - used_width;
                    let gap = total_gap / child_count as f32;
                    (gap, gap / 2.0)
                }
            };

            let mut x_offset = start_offset;
            for (i, child) in self.children.iter_mut().enumerate() {
                let span = spans[i].unwrap().0;
                let child_width = span as f32 * span_unit_width;
                let child_rect = Rect::new(
                    inner_rect.x + x_offset,
                    inner_rect.y,
                    child_width,
                    inner_rect.height,
                );
                child.layout(font_system, child_rect);
                x_offset += child_width + gap;
            }
        } else {
            // non-grid layout: respect individual child widths
            // first pass: calculate natural widths for each child
            let mut child_widths = Vec::new();
            let mut total_used_width = 0.0;
            let mut flexible_count = 0;

            for child in &mut self.children {
                let temp_rect = Rect::new(
                    inner_rect.x,
                    inner_rect.y,
                    999999.0, // unlimited width to get natural size
                    inner_rect.height,
                );
                child.layout(font_system, temp_rect);
                // calculate child's total "footprint" including any margin offset
                // footprint = (child.x - available.x) + child.width = margin_offset + content_width
                let margin_offset = child.rect().x - temp_rect.x;
                let footprint = margin_offset + child.rect().width;
                if footprint < 999999.0 {
                    child_widths.push(footprint);
                    total_used_width += footprint;
                } else {
                    child_widths.push(-1.0); // marker for flexible
                    flexible_count += 1;
                }
            }

            // distribute remaining space to flexible children
            let remaining_width = (total_width - total_used_width).max(0.0);
            let flexible_width = if flexible_count > 0 {
                remaining_width / flexible_count as f32
            } else {
                0.0
            };

            // replace markers with actual flexible width
            for w in &mut child_widths {
                if *w < 0.0 {
                    *w = flexible_width;
                }
            }

            let total_used: f32 = child_widths.iter().sum();

            let start_offset = match self.justify {
                Alignment::Start => 0.0,
                Alignment::Center => (total_width - total_used) / 2.0,
                Alignment::End => total_width - total_used,
                Alignment::SpaceBetween => 0.0,
                Alignment::SpaceAround => (total_width - total_used) / (child_count as f32 * 2.0),
            };

            let gap = match self.justify {
                Alignment::SpaceBetween if child_count > 1 => {
                    (total_width - total_used) / (child_count - 1) as f32
                }
                Alignment::SpaceAround => (total_width - total_used) / child_count as f32,
                _ => 0.0,
            };

            let mut x_offset = start_offset;
            for (_i, child) in self.children.iter_mut().enumerate() {
                // give child enough width for its needs - let it determine its own size
                let remaining = (total_width - x_offset).max(0.0);
                let child_rect = Rect::new(
                    inner_rect.x + x_offset,
                    inner_rect.y,
                    remaining,
                    inner_rect.height,
                );
                child.layout(font_system, child_rect);
                let child_end = child.rect().x + child.rect().width - inner_rect.x;
                x_offset = child_end + gap;
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
                    println!("[Row] Clicked. ID: {}", id);
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
}
