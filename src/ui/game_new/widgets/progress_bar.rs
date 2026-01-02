use super::Widget;
use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Color, Rect, Style};

pub struct ProgressBar {
    style: Style,
    rect: Rect,
    current_value: f32,
    max_value: f32,
    fill_color: Color,
    outline_color: Color,
}

impl ProgressBar {
    pub fn new(
        style: Style,
        current_value: f32,
        max_value: f32,
        fill_color: Color,
        outline_color: Color,
    ) -> Self {
        Self {
            style,
            rect: Rect::default(),
            current_value,
            max_value,
            fill_color,
            outline_color,
        }
    }
}

impl Widget for ProgressBar {
    fn layout(&mut self, _font_system: &mut FontSystem, available: Rect) {
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
    }

    fn update(&mut self, _ctx: &mut UiContext) -> bool {
        false
    }

    fn render(&self, renderer: &mut UiRenderer) {
        // Draw outline (full rect)
        let outline_rgba = self.outline_color.to_rgba();
        if outline_rgba[3] > 0.0 {
            renderer.draw_rect(self.rect, outline_rgba);
        }

        // Draw background (inset 1px)
        let bg_rgba = self.style.background.to_rgba();
        let border_width = 1.0;

        let inner_rect =
            self.rect
                .shrink_by(border_width, border_width, border_width, border_width);

        if inner_rect.width > 0.0 && inner_rect.height > 0.0 && bg_rgba[3] > 0.0 {
            renderer.draw_rect(inner_rect, bg_rgba);
        }

        // Draw fill (inset 1px, width scaled by pct)
        let fill_rgba = self.fill_color.to_rgba();
        if fill_rgba[3] > 0.0 {
            let pct = if self.max_value > 0.0 {
                (self.current_value / self.max_value).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let fill_width = inner_rect.width * pct;

            if fill_width > 0.0 && inner_rect.height > 0.0 {
                let fill_rect =
                    Rect::new(inner_rect.x, inner_rect.y, fill_width, inner_rect.height);
                renderer.draw_rect(fill_rect, fill_rgba);
            }
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }
}
