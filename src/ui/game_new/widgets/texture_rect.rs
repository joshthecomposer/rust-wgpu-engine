use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Color, Rect, Style};
use crate::ui::game_new::widgets::Widget;

pub struct TextureRect {
    pub texture_id: u32,
    pub style: Style,
    /// Optional color tint to apply to the texture.
    /// If None, white (no tint) is used.
    pub color: Option<Color>,
    rect: Rect,
}

impl TextureRect {
    pub fn new(texture_id: u32, style: Style) -> Self {
        Self {
            texture_id,
            style,
            color: None,
            rect: Rect::default(),
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

impl Widget for TextureRect {
    fn layout(&mut self, _font_system: &mut FontSystem, available: Rect) {
        // Apply fixed size or fill available space based on style
        // For simplicity, we'll start with the standard box model approach
        // simplified here as we delegate complex layout to the parent container usually
        // but we need to resolve our own rect based on available space and our style.

        // This is a simplified layout logic similar to BoxWidget
        let width = self
            .style
            .width
            .resolve_or(available.width, available.width);
        let height = self
            .style
            .height
            .resolve_or(available.height, available.height);

        self.rect = Rect {
            x: available.x,
            y: available.y,
            width,
            height,
        };
    }

    fn update(&mut self, ctx: &mut UiContext) -> bool {
        if self.rect.contains(ctx.mouse_pos()) {
            if ctx.is_click_start() {
                if let Some(id) = &self.style.id {
                    println!("[TextureRect] Clicked. ID: {}", id);
                    return true;
                }
                return false;
            }
        }
        false
    }

    fn render(&self, renderer: &mut UiRenderer) {
        let color_rgba = self.color.as_ref().map(|c| c.to_rgba());
        renderer.draw_textured_rect(self.rect, self.texture_id, color_rgba);
    }

    fn rect(&self) -> Rect {
        self.rect
    }
}
