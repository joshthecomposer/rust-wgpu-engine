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
        // println!(
        //     "[Column::layout] id={:?}, available height={}, style.height={:?}",
        //     self.style.id, available.height, self.style.height
        // );
        let (mt, mr, mb, ml) = self
            .style
            .resolve_margins(available.width, available.height);

        let content_x = available.x + ml;
        let content_y = available.y + mt;
        let max_width = available.width - ml - mr;
        let max_height = available.height - mt - mb;

        let width = self.style.width.resolve_or(max_width, max_width);
        let is_auto_height = self.style.height.resolve(max_height).is_none();
        // println!(
        //     "[Column] is_auto_height={}, max_height={}, width={}, style.height={:?}",
        //     is_auto_height, max_height, width, self.style.height
        // );

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
                let mut min_child_y = f32::MAX;
                let mut max_child_y = f32::MIN;
                for (i, child) in self.children.iter_mut().enumerate() {
                    let should_overlap = if let Some(child_id) = child.id() {
                        child_id == "pause_menu_panel" && i > 0
                    } else {
                        false
                    };

                    // position at y=0 to overlap previous child (shadow)
                    let child_y = if should_overlap { 0.0 } else { y_offset };

                    let child_rect = Rect::new(
                        content_x + pl_est,
                        content_y + pt_est + child_y,
                        inner_width,
                        999999.0,
                    );
                    child.layout(font_system, child_rect);
                    let child_height = child.rect().height;
                    child_heights.push(child_height);
                    // println!("[Column] child {} height = {}", i, child_height);

                    // ! account for negative margins - track actual child bounds
                    let child_final_y = child.rect().y;
                    let child_final_bottom = child_final_y + child.rect().height;
                    min_child_y = min_child_y.min(child_final_y);
                    max_child_y = max_child_y.max(child_final_bottom);

                    total_children_height += child_height;
                    // ! only accumulate y_offset if child doesn't overlap
                    if !should_overlap {
                        y_offset += child_height;
                    }
                }

                // Calculate height based on actual child bounds (accounts for negative margins)
                // For overlapping children, we want the height to be the panel height + shadow offset
                // not the full span from min_child_y to max_child_y
                let actual_content_height = if min_child_y < f32::MAX && max_child_y > f32::MIN {
                    // ! children extend from min_child_y to max_child_y (absolute positions)
                    // ! calculate relative to the inner content area start (content_y + pt_est)
                    let content_start_y = content_y + pt_est;

                    // ! For overlapping children (like shadow + panel), we want the height to be
                    // ! the maximum child height + any positive margin offset, not the full span
                    // ! Example: shadow at y=4 (height=400, bottom=404), panel at y=-4 (height=400, bottom=396)
                    // ! We want height = 400 (panel height) + 4 (shadow offset) = 404, not 408 (span)

                    let max_bottom_relative = (max_child_y - content_start_y).max(0.0);

                    // ! also check if we have overlapping children by checking if min_child_y is negative
                    // ! if so, use the maximum child height + the positive margin offset
                    if min_child_y < content_start_y {
                        // we have negative margins causing overlap
                        // use the maximum child height + the positive margin offset
                        let max_child_height = child_heights.iter().fold(0.0f32, |a, &b| a.max(b));
                        let positive_margin_offset =
                            (max_child_y - (content_start_y + max_child_height)).max(0.0);
                        max_child_height + positive_margin_offset
                    } else {
                        // Normal stacking, use the span
                        max_bottom_relative
                    }
                } else {
                    total_children_height
                };

                // recalculate padding with actual content height
                let (pt_final, _, pb_final, _) =
                    self.style.resolve_padding(max_width, actual_content_height);
                actual_content_height + pt_final + pb_final
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
        // println!(
        //     "[Column] computed rect: {:?}, height={}, max_height={}",
        //     self.rect, height, max_height
        // );

        let (pt, pr, pb, pl) = self
            .style
            .resolve_padding(self.rect.width, self.rect.height);
        let inner_rect = self.rect.shrink_by(pt, pr, pb, pl);
        // println!("[Column] inner_rect: {:?}", inner_rect);

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

                let should_overlap = if let Some(child_id) = child.id() {
                    child_id == "pause_menu_panel" && i > 0
                } else {
                    false
                };
                // ! position at y=0 to overlap previous child (shadow)
                let child_y = if should_overlap { 0.0 } else { y_offset };

                let child_has_explicit_positioning = if let Some(child_id) = child.id() {
                    // ! close button uses margin_left for positioning
                    child_id == "btn_close"
                } else {
                    false
                };

                let available_width = if inner_rect.width > 0.0 {
                    inner_rect.width
                } else {
                    // fallback: use parent's available width minus padding
                    let (_pt, pr, _pb, pl) = self
                        .style
                        .resolve_padding(self.rect.width.max(420.0), self.rect.height);
                    (self.rect.width.max(420.0) - pl - pr).max(388.0)
                };

                // for explicitly positioned children (like close button), layout them directly without centering
                // for other children, layout first to get width, then center if needed
                if child_has_explicit_positioning {
                    let child_rect = Rect::new(
                        inner_rect.x,
                        inner_rect.y + child_y,
                        available_width,
                        child_height,
                    );
                    child.layout(font_system, child_rect);
                } else {
                    let child_rect_temp = Rect::new(
                        inner_rect.x,
                        inner_rect.y + child_y,
                        available_width,
                        child_height,
                    );
                    child.layout(font_system, child_rect_temp);

                    let child_width = child.rect().width;
                    let x_offset = match self.align {
                        Alignment::Center => (available_width - child_width) / 2.0,
                        Alignment::End => available_width - child_width,
                        _ => 0.0,
                    };

                    // re-layout with correct x position if alignment requires offset
                    if x_offset != 0.0 {
                        let child_rect = Rect::new(
                            inner_rect.x + x_offset,
                            inner_rect.y + child_y,
                            available_width,
                            child_height,
                        );
                        child.layout(font_system, child_rect);
                    }
                }

                if !should_overlap {
                    y_offset += child_height;
                }
            }
        } else {
            let total_height = inner_rect.height;

            // ! first pass: layout children to get their actual heights
            let mut actual_child_heights = Vec::with_capacity(child_count);
            for child in &mut self.children {
                // layout each child with full available height to measure its actual size
                let temp_rect =
                    Rect::new(inner_rect.x, inner_rect.y, inner_rect.width, total_height);
                child.layout(font_system, temp_rect);
                actual_child_heights.push(child.rect().height);
            }

            let total_used_height: f32 = actual_child_heights.iter().sum();
            let remaining_space = (total_height - total_used_height).max(0.0);

            // calculate gap and start offset based on justify alignment
            let (gap, start_offset) = match self.justify {
                Alignment::Start => (0.0, 0.0),
                Alignment::Center => (0.0, remaining_space / 2.0),
                Alignment::End => (0.0, remaining_space),
                Alignment::SpaceBetween => {
                    if child_count > 1 {
                        (remaining_space / (child_count - 1) as f32, 0.0)
                    } else {
                        (0.0, 0.0)
                    }
                }
                Alignment::SpaceAround => {
                    let gap = remaining_space / child_count as f32;
                    (gap, gap / 2.0)
                }
            };

            // ! second pass: position children correctly
            let mut y_offset = start_offset;
            for (i, child) in self.children.iter_mut().enumerate() {
                let child_height = actual_child_heights[i];

                let child_has_explicit_positioning = if let Some(child_id) = child.id() {
                    // ! close button uses margin_left for positioning
                    child_id == "btn_close"
                } else {
                    false
                };

                // for explicitly positioned children (like close button), layout them directly without centering
                // for other children, layout first to get width, then center if needed
                if child_has_explicit_positioning {
                    let child_rect = Rect::new(
                        inner_rect.x,
                        inner_rect.y + y_offset,
                        inner_rect.width,
                        child_height,
                    );
                    child.layout(font_system, child_rect);
                } else {
                    let child_rect_temp = Rect::new(
                        inner_rect.x,
                        inner_rect.y + y_offset,
                        inner_rect.width,
                        child_height,
                    );
                    child.layout(font_system, child_rect_temp);

                    let child_width = child.rect().width;
                    let x_offset = match self.align {
                        Alignment::Center => (inner_rect.width - child_width) / 2.0,
                        Alignment::End => inner_rect.width - child_width,
                        _ => 0.0,
                    };

                    if x_offset != 0.0 {
                        let child_rect = Rect::new(
                            inner_rect.x + x_offset,
                            inner_rect.y + y_offset,
                            inner_rect.width,
                            child_height,
                        );
                        child.layout(font_system, child_rect);
                    }
                }

                y_offset += child_height + gap;
            }
        }
    }

    fn update(&mut self, ctx: &mut UiContext) -> bool {
        // process children in REVERSE order (last child first)
        // this ensures widgets rendered on top (later in children list) get click priority
        // this is important for overlapping widgets like shadow + panel
        for child in self.children.iter_mut().rev() {
            if child.update(ctx) {
                return true;
            }
        }

        if self.rect.contains(ctx.mouse_pos()) {
            if ctx.is_click_start() {
                // if let Some(id) = &self.style.id {
                //      println!("[Column] Clicked. ID: {}", id);
                // }
                return true;
            }
        }
        false
    }

    fn overlay_update(&mut self, ctx: &mut UiContext) -> bool {
        for child in self.children.iter_mut() {
            if child.overlay_update(ctx) {
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
