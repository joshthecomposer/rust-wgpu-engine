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

    fn grid_span(&self) -> Option<GridSpan> {
        Some(self.span)
    }
}
