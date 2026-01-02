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
        let height = self.style.height.resolve_or(max_height, max_height);

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
            // fallback: equal distribution (original behavior)
            let (gap, start_offset) = match self.justify {
                Alignment::Start => (0.0, 0.0),
                Alignment::Center => {
                    let child_width = total_width / child_count as f32;
                    (0.0, (total_width - child_width * child_count as f32) / 2.0)
                }
                Alignment::End => {
                    let child_width = total_width / child_count as f32;
                    (0.0, total_width - child_width * child_count as f32)
                }
                Alignment::SpaceBetween => {
                    if child_count > 1 {
                        let child_width = total_width / child_count as f32;
                        let gap = (total_width - child_width * child_count as f32)
                            / (child_count - 1) as f32;
                        (gap, 0.0)
                    } else {
                        (0.0, 0.0)
                    }
                }
                Alignment::SpaceAround => {
                    let child_width = total_width / child_count as f32;
                    let total_gap = total_width - child_width * child_count as f32;
                    let gap = total_gap / child_count as f32;
                    (gap, gap / 2.0)
                }
            };

            let mut x_offset = start_offset;
            for child in &mut self.children {
                let child_width = total_width / child_count as f32;
                let child_rect = Rect::new(
                    inner_rect.x + x_offset,
                    inner_rect.y,
                    child_width,
                    inner_rect.height,
                );
                child.layout(font_system, child_rect);
                x_offset += child_width + gap;
            }
        }
    }

    fn update(&mut self, ctx: &mut UiContext) -> bool {
        for child in &mut self.children {
            if child.update(ctx) {
                return true;
            }
        }
        false
    }

    fn render(&self, renderer: &mut UiRenderer) {
        let color = self.style.background.to_rgba();
        if color[3] > 0.0 {
            renderer.draw_rect(self.rect, color);
        }

        for child in &self.children {
            child.render(renderer);
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }
}
