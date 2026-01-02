use super::vertex::UiVertex;
use crate::ui::game_new::styles::Rect;

pub struct RenderBatch {
    pub vertices: Vec<UiVertex>,
    pub indices: Vec<u32>,
}

impl RenderBatch {
    pub fn new() -> Self {
        Self {
            vertices: Vec::with_capacity(1024),
            indices: Vec::with_capacity(2048),
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    pub fn push_rect(&mut self, rect: Rect, color: [f32; 4]) {
        let base_idx = self.vertices.len() as u32;

        let x0 = rect.x;
        let y0 = rect.y;
        let x1 = rect.x + rect.width;
        let y1 = rect.y + rect.height;

        self.vertices.push(UiVertex::new(x0, y0, color)); // top-left
        self.vertices.push(UiVertex::new(x1, y0, color)); // top-right
        self.vertices.push(UiVertex::new(x1, y1, color)); // bottom-right
        self.vertices.push(UiVertex::new(x0, y1, color)); // bottom-left

        self.indices.push(base_idx);
        self.indices.push(base_idx + 1);
        self.indices.push(base_idx + 2);

        self.indices.push(base_idx);
        self.indices.push(base_idx + 2);
        self.indices.push(base_idx + 3);
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }
}

impl Default for RenderBatch {
    fn default() -> Self {
        Self::new()
    }
}
