use std::sync::Arc;
use winit::window::Window;

pub struct Platform {
    pub fb_width: u32,
    pub fb_height: u32,
    pub window: Option<Arc<Window>>,
}
