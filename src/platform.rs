use std::ffi::CStr;

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

#[allow(dead_code)]
#[derive(PartialEq, Copy, Clone)]
pub enum CursorMode {
    Normal,
    Hidden,
    #[allow(dead_code)]
    Disabled,
}

pub struct Platform {
    pub window: Window,
    pub gl_context: PossiblyCurrentContext,
    pub surface: Surface<WindowSurface>,
    pub capabilities: GlCapabilities,
    pub fb_width: u32,
    pub fb_height: u32,
    pub scale_factor: f64,
    pub cursor_mode: CursorMode,
    pub display: Display,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformBackend {
    NativeGlutin,
    WebCanvas,
}

#[derive(Clone, Debug)]
pub struct GlCapabilities {
    pub gl_version: String,
    pub glsl_version: String,
    pub vendor: String,
    pub renderer: String,
    pub extensions: Vec<String>,
    pub is_gles_like: bool,
    pub supports_float_color_buffer: bool,
    pub supports_msaa_float_renderbuffer: bool,
    pub supports_clamp_to_border: bool,
    pub supports_buffer_mapping: bool,
    pub supports_instancing: bool,
    pub supports_mrt: bool,
}

impl GlCapabilities {
    #[cfg(target_arch = "wasm32")]
    pub fn webgl2_defaults() -> Self {
        Self {
            gl_version: "WebGL 2.0".to_string(),
            glsl_version: "GLSL ES 3.00".to_string(),
            vendor: "Browser".to_string(),
            renderer: "WebGL2 canvas".to_string(),
            extensions: Vec::new(),
            is_gles_like: true,
            supports_float_color_buffer: false,
            supports_msaa_float_renderbuffer: false,
            supports_clamp_to_border: false,
            supports_buffer_mapping: false,
            supports_instancing: true,
            supports_mrt: true,
        }
    }

    pub fn query_current_context() -> Self {
        let gl_version = gl_string(gl::VERSION);
        let glsl_version = gl_string(gl::SHADING_LANGUAGE_VERSION);
        let vendor = gl_string(gl::VENDOR);
        let renderer = gl_string(gl::RENDERER);
        let extensions = gl_extensions();
        let is_gles_like = gl_version.contains("OpenGL ES") || gl_version.contains("WebGL");
        let max_color_attachments = gl_integer(gl::MAX_COLOR_ATTACHMENTS);
        let max_draw_buffers = gl_integer(gl::MAX_DRAW_BUFFERS);
        let max_samples = gl_integer(gl::MAX_SAMPLES);

        let has_extension = |name: &str| extensions.iter().any(|extension| extension == name);
        let has_any_extension = |names: &[&str]| names.iter().any(|name| has_extension(name));
        let is_webgl_like = gl_version.contains("WebGL");
        let supports_float_color_buffer = !is_gles_like
            || has_any_extension(&[
                "GL_EXT_color_buffer_float",
                "EXT_color_buffer_float",
                "GL_ARB_color_buffer_float",
            ]);
        let supports_msaa_float_renderbuffer = supports_float_color_buffer
            && ((!is_gles_like && max_samples > 0)
                || has_any_extension(&[
                    "GL_EXT_multisampled_render_to_texture",
                    "GL_EXT_multisampled_render_to_texture2",
                    "GL_IMG_multisampled_render_to_texture",
                ]));
        let supports_clamp_to_border = !is_gles_like
            || has_any_extension(&[
                "GL_ARB_texture_border_clamp",
                "GL_EXT_texture_border_clamp",
                "GL_OES_texture_border_clamp",
                "GL_NV_texture_border_clamp",
            ]);
        let supports_buffer_mapping = !is_webgl_like
            && (!is_gles_like
                || has_any_extension(&[
                    "GL_OES_mapbuffer",
                    "GL_EXT_map_buffer_range",
                    "GL_NV_map_buffer_range",
                ])
                || gl_version.contains("OpenGL ES 3"));
        let supports_instancing = !is_gles_like
            || has_any_extension(&[
                "GL_ANGLE_instanced_arrays",
                "GL_EXT_instanced_arrays",
                "GL_NV_instanced_arrays",
            ])
            || gl_version.contains("OpenGL ES 3");
        let supports_mrt = max_color_attachments >= 2 && max_draw_buffers >= 2;

        Self {
            gl_version,
            glsl_version,
            vendor,
            renderer,
            extensions,
            is_gles_like,
            supports_float_color_buffer,
            supports_msaa_float_renderbuffer,
            supports_clamp_to_border,
            supports_buffer_mapping,
            supports_instancing,
            supports_mrt,
        }
    }

    pub fn log_startup_report(&self) {
        println!("GL capabilities:");
        println!("  Version: {}", self.gl_version);
        println!("  GLSL: {}", self.glsl_version);
        println!("  Vendor: {}", self.vendor);
        println!("  Renderer: {}", self.renderer);
        println!("  Extensions: {}", self.extensions.len());
        println!("  GLES/WebGL-like: {}", self.is_gles_like);
        println!(
            "  Float color buffer: {}",
            self.supports_float_color_buffer
        );
        println!(
            "  MSAA float renderbuffer: {}",
            self.supports_msaa_float_renderbuffer
        );
        println!("  Clamp to border: {}", self.supports_clamp_to_border);
        println!("  Buffer mapping: {}", self.supports_buffer_mapping);
        println!("  Instancing: {}", self.supports_instancing);
        println!("  Multiple render targets: {}", self.supports_mrt);
    }
}

#[cfg(target_arch = "wasm32")]
pub mod web_canvas {
    use super::{CursorMode, GlCapabilities, PlatformBackend};

    #[allow(dead_code)]
    pub struct WebCanvasPlatform {
        pub capabilities: GlCapabilities,
        pub fb_width: u32,
        pub fb_height: u32,
        pub scale_factor: f64,
        pub cursor_mode: CursorMode,
    }

    impl WebCanvasPlatform {
        #[allow(dead_code)]
        pub fn backend(&self) -> PlatformBackend {
            PlatformBackend::WebCanvas
        }

        #[allow(dead_code)]
        pub fn placeholder(w: u32, h: u32) -> Self {
            Self {
                capabilities: GlCapabilities::webgl2_defaults(),
                fb_width: w,
                fb_height: h,
                scale_factor: 1.0,
                cursor_mode: CursorMode::Normal,
            }
        }
    }
}

impl Platform {
    #[allow(dead_code)]
    pub fn backend(&self) -> PlatformBackend {
        PlatformBackend::NativeGlutin
    }

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

        let capabilities = GlCapabilities::query_current_context();
        capabilities.log_startup_report();

        unsafe { gl_call!(gl::Enable(gl::MULTISAMPLE)) };

        let size = window.inner_size();
        let scale_factor = window.scale_factor();

        let mut platform = Self {
            window,
            gl_context,
            surface,
            capabilities,
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

fn gl_string(name: u32) -> String {
    unsafe {
        let value = gl::GetString(name);
        if value.is_null() {
            return "Unavailable".to_string();
        }

        CStr::from_ptr(value.cast())
            .to_string_lossy()
            .into_owned()
    }
}

fn gl_extensions() -> Vec<String> {
    let extension_count = gl_integer(gl::NUM_EXTENSIONS);
    if extension_count > 0 {
        return (0..extension_count)
            .filter_map(|index| unsafe {
                let extension = gl::GetStringi(gl::EXTENSIONS, index as u32);
                if extension.is_null() {
                    None
                } else {
                    Some(CStr::from_ptr(extension.cast()).to_string_lossy().into_owned())
                }
            })
            .collect();
    }

    gl_string(gl::EXTENSIONS)
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

fn gl_integer(name: u32) -> i32 {
    let mut value = 0;
    unsafe {
        gl::GetIntegerv(name, &mut value);
    }
    value
}
