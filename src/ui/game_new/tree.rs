use super::context::UiContext;
use super::render::UiRenderer;
use super::styles::Rect;
use super::widgets::Widget;

pub struct UiTree {
    root: Option<Box<dyn Widget>>,
    screen_rect: Rect,
    needs_layout: bool,
}

impl UiTree {
    pub fn new() -> Self {
        Self {
            root: None,
            screen_rect: Rect::default(),
            needs_layout: true,
        }
    }

    pub fn set_root(&mut self, root: Box<dyn Widget>) {
        self.root = Some(root);
        self.needs_layout = true;
    }

    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        let new_rect = Rect::new(0.0, 0.0, width, height);
        if self.screen_rect.width != new_rect.width || self.screen_rect.height != new_rect.height {
            self.screen_rect = new_rect;
            self.needs_layout = true;
        }
    }

    pub fn layout(&mut self) {
        if !self.needs_layout {
            return;
        }

        if let Some(root) = &mut self.root {
            root.layout(self.screen_rect);
        }

        self.needs_layout = false;
    }

    pub fn force_layout(&mut self) {
        self.needs_layout = true;
        self.layout();
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
}

impl Default for UiTree {
    fn default() -> Self {
        Self::new()
    }
}

