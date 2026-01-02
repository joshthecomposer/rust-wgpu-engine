use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Rect, Style};

use super::Widget;

pub struct BoxWidget {
    pub style: Style,
    rect: Rect,
}

impl BoxWidget {
    pub fn new(style: Style) -> Self {
        Self {
            style,
            rect: Rect::default(),
        }
    }
}

use crate::ui::game_new::font_system::FontSystem;

impl Widget for BoxWidget {
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

    fn update(&mut self, ctx: &mut UiContext) -> bool {
        // Check if mouse is over this widget
        if self.rect.contains(ctx.mouse_pos()) {
            // Check if processed a click
            if ctx.is_click_start() {
                if let Some(id) = &self.style.id {
                    println!("[BoxWidget] Clicked. ID: {}", id);
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
    }

    fn rect(&self) -> Rect {
        self.rect
    }
}
