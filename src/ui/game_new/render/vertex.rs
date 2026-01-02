#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct UiVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

impl UiVertex {
    pub fn new(x: f32, y: f32, color: [f32; 4], uv: [f32; 2]) -> Self {
        Self {
            position: [x, y],
            color,
            uv,
        }
    }
}

