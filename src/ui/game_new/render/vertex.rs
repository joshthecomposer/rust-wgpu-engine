#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct UiVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub uv: [f32; 2],
    pub rect_bounds: [f32; 4],
    pub border_radius: f32,
}

impl UiVertex {
    pub fn new(
        x: f32,
        y: f32,
        color: [f32; 4],
        uv: [f32; 2],
        rect_bounds: [f32; 4],
        border_radius: f32,
    ) -> Self {
        Self {
            position: [x, y],
            color,
            uv,
            rect_bounds,
            border_radius,
        }
    }
}
