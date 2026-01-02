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

        // use the center of a 1x1 white pixel in the texture atlas to avoid edge filtering artifacts.
        let uv = [0.5, 0.5];

        self.vertices.push(UiVertex::new(x0, y0, color, uv)); // top-left
        self.vertices.push(UiVertex::new(x1, y0, color, uv)); // top-right
        self.vertices.push(UiVertex::new(x1, y1, color, uv)); // bottom-right
        self.vertices.push(UiVertex::new(x0, y1, color, uv)); // bottom-left

        self.indices.push(base_idx);
        self.indices.push(base_idx + 1);
        self.indices.push(base_idx + 2);

        self.indices.push(base_idx);
        self.indices.push(base_idx + 2);
        self.indices.push(base_idx + 3);
    }

    pub fn push_textured_rect(&mut self, rect: Rect, uv: [f32; 4], color: [f32; 4]) {
        let base_idx = self.vertices.len() as u32;

        let x0 = rect.x;
        let y0 = rect.y;
        let x1 = rect.x + rect.width;
        let y1 = rect.y + rect.height;

        let u0 = uv[0]; // min u
        let v0 = uv[1]; // min v
        let u1 = uv[2]; // max u
        let v1 = uv[3]; // max v

        // Top-left
        self.vertices.push(UiVertex::new(x0, y0, color, [u0, v0]));
        // Top-right
        self.vertices.push(UiVertex::new(x1, y0, color, [u1, v0]));
        // Bottom-right
        self.vertices.push(UiVertex::new(x1, y1, color, [u1, v1]));
        // Bottom-left
        self.vertices.push(UiVertex::new(x0, y1, color, [u0, v1]));

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
