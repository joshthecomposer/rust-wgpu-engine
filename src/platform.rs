use glutin::{
    config::{Config, ConfigTemplateBuilder},
    context::{
        ContextApi, ContextAttributesBuilder, GlProfile, NotCurrentContext, PossiblyCurrentContext,
        Version,
    },
    display::{Display, GetGlDisplay},
    prelude::*,
    surface::{GlSurface, Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface},
};

use glutin_winit::{ApiPreference, DisplayBuilder, GlWindow};
use winit::raw_window_handle::HasWindowHandle;

use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{CursorGrabMode, Window, WindowAttributes},
};

use crate::gl_call;

#[derive(PartialEq, Copy, Clone)]
pub enum CursorMode {
    Normal,
    Hidden,
    Disabled,
}

pub struct Platform {
    pub window: Window,
    pub gl_context: PossiblyCurrentContext,
    pub surface: Surface<WindowSurface>,
    pub fb_width: u32,
    pub fb_height: u32,
    pub scale_factor: f64,
    pub cursor_mode: CursorMode,
    pub display: Display,
}

impl Platform {
    pub fn new(title: &str, w: u32, h: u32, _vsync: bool) -> (Self, EventLoop<()>) {
        let event_loop = EventLoop::new().expect("Failed to create EventLoop");

        let window_attrs: WindowAttributes = Window::default_attributes()
            .with_title(title.to_string())
            .with_inner_size(LogicalSize::new(w, h))
            .with_resizable(true);

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_depth_size(24)
            .with_stencil_size(8);
        //.with_multisampling(16);

        let display_builder = DisplayBuilder::new()
            .with_preference(ApiPreference::FallbackEgl) // Prefer desktop GL
            .with_window_attributes(Some(window_attrs));

        let (window_opt, config) = display_builder
            .build(&event_loop, template, pick_config)
            .expect("Failed to build glutin display + window");

        let window = window_opt.expect("DisplayBuilder did not create a window");
        let display = config.display();

        let raw_window_handle = Some(window.window_handle().unwrap().as_raw());

        let context_attrs = ContextAttributesBuilder::new()
            .with_profile(GlProfile::Core)
            .with_context_api(ContextApi::OpenGl(Some(Version::new(4, 6))))
            .build(raw_window_handle);

        let fallback_attrs = ContextAttributesBuilder::new()
            .with_profile(GlProfile::Core)
            .with_context_api(ContextApi::Gles(Some(Version::new(3, 2))))
            .build(raw_window_handle);

        let not_current: NotCurrentContext = unsafe {
            display
                .create_context(&config, &context_attrs)
                .or_else(|_| display.create_context(&config, &fallback_attrs))
                .expect("Failed to create OpenGL context")
        };

        let attrs = window
            .build_surface_attributes(SurfaceAttributesBuilder::<WindowSurface>::new())
            .expect("Failed to build surface attributes");

        let surface = unsafe {
            display
                .create_window_surface(&config, &attrs)
                .expect("Failed to create window surface")
        };

        let gl_context = not_current
            .make_current(&surface)
            .expect("Failed to make GL context current");

        //if vsync {
        //    let _ = surface.set_swap_interval(
        //        &gl_context,
        //        SwapInterval::Wait(std::num::NonZeroU32::new(1).unwrap()),
        //    );
        //} else {
        //    let _ = surface.set_swap_interval(&gl_context, SwapInterval::DontWait);
        //}

        match surface.set_swap_interval(&gl_context, SwapInterval::DontWait) {
            Ok(_) => println!("VSync disabled"),
            Err(e) => eprintln!("Failed to disable vsync: {:?}", e),
        }

        gl::load_with(|symbol| {
            display.get_proc_address(&std::ffi::CString::new(symbol).unwrap()) as *const _
        });

        unsafe { gl_call!(gl::Enable(gl::MULTISAMPLE)) };

        let size = window.inner_size();
        let scale_factor = window.scale_factor();

        let mut platform = Self {
            window,
            gl_context,
            surface,
            fb_width: size.width,
            fb_height: size.height,
            scale_factor,
            cursor_mode: CursorMode::Normal,
            display,
        };

        platform.set_cursor_mode(CursorMode::Hidden);

        (platform, event_loop)
    }

    pub fn set_cursor_mode(&mut self, mode: CursorMode) {
        if self.cursor_mode == mode {
            return;
        }

        match mode {
            CursorMode::Normal => {
                let _ = self.window.set_cursor_grab(CursorGrabMode::None);
                self.window.set_cursor_visible(true);
            }
            CursorMode::Hidden => {
                let _ = self.window.set_cursor_grab(CursorGrabMode::Locked);
                self.window.set_cursor_visible(false);
            }
            CursorMode::Disabled => {
                let _ = self.window.set_cursor_grab(CursorGrabMode::Locked);
                self.window.set_cursor_visible(false);
            }
        }

        self.cursor_mode = mode;
    }
}

fn pick_config<'a>(configs: Box<dyn Iterator<Item = Config> + 'a>) -> Config {
    configs
        .reduce(|best, config| {
            if config.num_samples() > best.num_samples() {
                config
            } else {
                best
            }
        })
        .expect("No GL configs found")
}
