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
    text_size: (f32, f32), // Cached text dimensions for centering
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
            text_size: (0.0, 0.0),
        }
    }

    pub fn set_value(&mut self, value: f32) {
        self.current_value = value;
    }

    pub fn set_max_value(&mut self, max: f32) {
        self.max_value = max;
    }
}

impl Widget for ProgressBar {
    fn layout(&mut self, font_system: &mut FontSystem, available: Rect) {
        let (mt, mr, mb, ml) = self
            .style
            .resolve_margins(available.width, available.height);

        let content_x = available.x + ml;
        let content_y = available.y + mt;
        let max_width = available.width - ml - mr;
        let max_height = available.height - mt - mb;

        let width = match self.style.width.resolve(max_width) {
            Some(w) => w.min(available.width),
            None => max_width,
        };
        let height = match self.style.height.resolve(max_height) {
            Some(h) => h.min(available.height),
            None => max_height,
        };

        self.rect = Rect::new(content_x, content_y, width, height);

        // measure text for centering during render
        let text = format!(
            "{} / {}",
            self.current_value.round() as i32,
            self.max_value.round() as i32
        );
        self.text_size = font_system.measure_text(&text, 11.0, None);
    }

    fn update(&mut self, _ctx: &mut UiContext) -> bool {
        false
    }

    fn render(&self, renderer: &mut UiRenderer) {
        // Draw outline (full rect)
        let outline_rgba = self.outline_color.to_rgba();
        if outline_rgba[3] > 0.0 {
            renderer.draw_rect(self.rect, outline_rgba, 0.0);
        }

        // Draw background (inset 1px)
        let bg_rgba = self.style.background.to_rgba();
        let border_width = 1.0;

        let inner_rect =
            self.rect
                .shrink_by(border_width, border_width, border_width, border_width);

        if inner_rect.width > 0.0 && inner_rect.height > 0.0 && bg_rgba[3] > 0.0 {
            renderer.draw_rect(inner_rect, bg_rgba, 0.0);
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
                renderer.draw_rect(fill_rect, fill_rgba, 0.0);
            }
        }

        // draw centered text overlay: "current / max"
        let text = format!(
            "{} / {}",
            self.current_value.round() as i32,
            self.max_value.round() as i32
        );
        let font_size = 11.0;
        let text_color = [0.996, 0.953, 0.780, 1.0]; // Theme.highlight (#fef3c7)

        // use cached text_size from layout, or approximate if not available
        let (text_width, text_height) = self.text_size;
        let text_x = self.rect.x + (self.rect.width - text_width) / 2.0;
        // vertical center: account for font baseline (text renders from top-left)
        let text_y = self.rect.y + (self.rect.height - text_height) / 2.0;

        renderer.draw_text(&text, text_x, text_y, font_size, text_color, None);
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
