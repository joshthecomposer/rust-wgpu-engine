use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use slint::platform::{
    software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
    Platform, PlatformError, WindowAdapter,
};

// Global storage for windows created by the platform.
// This allows retrieving windows after component creation.
thread_local! {
    static CREATED_WINDOWS: RefCell<Vec<Rc<MinimalSoftwareWindow>>> = RefCell::new(Vec::new());
}

/// Custom Slint platform that uses MinimalSoftwareWindow for rendering.
/// This allows us to use our own winit window and event loop instead of Slint's.
///
/// Each component gets its own window. Windows are stored in CREATED_WINDOWS
/// and can be retrieved via `get_last_created_window()`.
pub struct SlintPlatform {
    default_size: (u32, u32),
    scale_factor: f32,
    start_time: Instant,
}

impl SlintPlatform {
    pub fn new(width: u32, height: u32, scale_factor: f32) -> Self {
        Self {
            default_size: (width, height),
            scale_factor,
            start_time: Instant::now(),
        }
    }
}

impl Platform for SlintPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        // create a new window for each component
        let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);

        // IMPORTANT: set scale factor FIRST, before set_size, so the logical size is computed correctly
        window.dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged {
            scale_factor: self.scale_factor,
        });

        // now set the physical size - this will compute logical size using the scale factor
        window.set_size(slint::PhysicalSize::new(
            self.default_size.0,
            self.default_size.1,
        ));

        // store the window so it can be retrieved later
        CREATED_WINDOWS.with(|windows| {
            windows.borrow_mut().push(window.clone());
        });

        Ok(window)
    }

    fn duration_since_start(&self) -> std::time::Duration {
        Instant::now().duration_since(self.start_time)
    }

    // we don't need run_event_loop - we're providing our own via winit
    // the default implementation will panic if called, which is fine since we won't call it.
}

/// Initialize the Slint platform. Must be called BEFORE creating any Slint components.
pub fn init_slint_platform(width: u32, height: u32, scale_factor: f32) {
    let platform = SlintPlatform::new(width, height, scale_factor);
    slint::platform::set_platform(Box::new(platform))
        .expect("Failed to set Slint platform - was it already set?");
}

/// Get the last window created by the platform (i.e., the window for the most recently created component).
/// This should be called immediately after creating a Slint component to get its window.
pub fn get_last_created_window() -> Option<Rc<MinimalSoftwareWindow>> {
    CREATED_WINDOWS.with(|windows| windows.borrow().last().cloned())
}
