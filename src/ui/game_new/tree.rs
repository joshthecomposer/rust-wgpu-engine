use super::context::UiContext;
use super::font_system::FontSystem;
use super::render::UiRenderer;
use super::styles::Rect;
use super::widgets::Widget;

pub struct UiTree {
    root: Option<Box<dyn Widget>>,
    screen_rect: Rect,
    needs_layout: bool,
    /// x offset for rendering (used to position tree at specific screen location)
    offset_x: f32,
    /// y offset for rendering
    offset_y: f32,
}

impl UiTree {
    pub fn new() -> Self {
        Self {
            root: None,
            screen_rect: Rect::default(),
            needs_layout: true,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }

    pub fn set_root(&mut self, root: Box<dyn Widget>) {
        self.root = Some(root);
        self.needs_layout = true;
    }

    /// Set the position offset for rendering.
    /// The tree will render at (offset_x, offset_y) instead of (0, 0).
    pub fn set_offset(&mut self, x: f32, y: f32) {
        self.offset_x = x;
        self.offset_y = y;
    }

    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        let new_rect = Rect::new(0.0, 0.0, width, height);
        if self.screen_rect.width != new_rect.width || self.screen_rect.height != new_rect.height {
            self.screen_rect = new_rect;
            self.needs_layout = true;
        }
    }

    pub fn layout(&mut self, font_system: &mut FontSystem) {
        if !self.needs_layout {
            return;
        }

        if let Some(root) = &mut self.root {
            // apply offset to the rect for positioning
            let rect = Rect::new(
                self.offset_x,
                self.offset_y,
                self.screen_rect.width,
                self.screen_rect.height,
            );
            root.layout(font_system, rect);
        }

        self.needs_layout = false;
    }

    pub fn force_layout(&mut self) {
        self.needs_layout = true;
    }

    pub fn update(&mut self, ctx: &mut UiContext) -> bool {
        if let Some(root) = &mut self.root {
            root.update(ctx)
        } else {
            false
        }
    }

    pub fn render(&self, renderer: &mut UiRenderer) {
        if let Some(root) = &self.root {
            root.render(renderer);
        }
    }

    pub fn find_widget_mut(&mut self, id: &str) -> Option<&mut dyn Widget> {
        if let Some(root) = &mut self.root {
            root.find_widget_mut(id)
        } else {
            None
        }
    }
}

impl Default for UiTree {
    fn default() -> Self {
        Self::new()
    }
}
