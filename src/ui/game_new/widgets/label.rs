use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Rect, Style};
use crate::ui::game_new::widgets::text::Text;
use crate::ui::game_new::widgets::Widget;

/// A convenience widget that wraps Text.
///
/// Label is essentially a Text widget but semantic naming can differ or be extended later.
pub struct Label {
    text: Text,
}

impl Label {
    pub fn new(content: String, style: Style) -> Self {
        Self {
            text: Text::new(content, style),
        }
    }
}

impl Widget for Label {
    fn layout(&mut self, font_system: &mut FontSystem, available: Rect) {
        self.text.layout(font_system, available);
    }

    fn update(&mut self, ctx: &mut UiContext) -> bool {
        self.text.update(ctx)
    }

    fn render(&self, renderer: &mut UiRenderer) {
        self.text.render(renderer);
    }

    fn rect(&self) -> Rect {
        self.text.rect()
    }
}
