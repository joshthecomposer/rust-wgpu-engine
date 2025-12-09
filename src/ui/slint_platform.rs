use std::rc::Rc;
use std::time::Instant;

use slint::platform::{
    software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
    Platform, PlatformError, WindowAdapter,
};

/// Custom Slint platform that uses MinimalSoftwareWindow for rendering.
/// This allows us to use our own winit window and event loop instead of Slint's.
pub struct SlintPlatform {
    window: Rc<MinimalSoftwareWindow>,
    start_time: Instant,
}

impl SlintPlatform {
    pub fn new(width: u32, height: u32) -> (Self, Rc<MinimalSoftwareWindow>) {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
        window.set_size(slint::PhysicalSize::new(width, height));

        let platform = Self {
            window: window.clone(),
            start_time: Instant::now(),
        };

        (platform, window)
    }
}

impl Platform for SlintPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> std::time::Duration {
        Instant::now().duration_since(self.start_time)
    }

    // we don't need run_event_loop - we're providing our own via winit
    // the default implementation will panic if called, which is fine since we won't call it.
}

/// Initialize the Slint platform. Must be called BEFORE creating any Slint components.
/// Returns the MinimalSoftwareWindow that can be used for rendering.
pub fn init_slint_platform(width: u32, height: u32) -> Rc<MinimalSoftwareWindow> {
    let (platform, window) = SlintPlatform::new(width, height);
    slint::platform::set_platform(Box::new(platform))
        .expect("Failed to set Slint platform - was it already set?");
    window
}
