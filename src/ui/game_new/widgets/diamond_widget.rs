use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Rect, Style};

use super::Widget;

pub struct DiamondWidget {
    pub style: Style,
    rect: Rect,
}

impl DiamondWidget {
    pub fn new(style: Style) -> Self {
        Self {
            style,
            rect: Rect::default(),
        }
    }
}

use crate::ui::game_new::font_system::FontSystem;

impl Widget for DiamondWidget {
    fn layout(&mut self, _font_system: &mut FontSystem, available: Rect) {
        let (mt, _mr, _mb, ml) = self
            .style
            .resolve_margins(available.width, available.height);

        let content_x = available.x + ml;
        let content_y = available.y + mt;

        // for diamonds with explicit Px dimensions, don't clamp to available space
        // the parent should allocate enough space for content + margins
        let width = self
            .style
            .width
            .resolve_or(available.width, available.width);
        let height = self
            .style
            .height
            .resolve_or(available.height, available.height);

        self.rect = Rect::new(content_x, content_y, width, height);
    }

    fn update(&mut self, ctx: &mut UiContext) -> bool {
        if self.rect.contains(ctx.mouse_pos()) {
            if ctx.is_click_start() {
                return true;
            }
        }
        false
    }

    fn render(&self, renderer: &mut UiRenderer) {
        let color = self.style.background.to_rgba();
        if color[3] > 0.0 {
            renderer.draw_diamond(self.rect, color);
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

    fn find_widget_mut(&mut self, id: &str) -> Option<&mut dyn Widget> {
        if self.id() == Some(id) {
            return Some(self);
        }
        None
    }
}
