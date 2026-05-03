#[cfg(not(target_arch = "wasm32"))]
use std::ffi::CStr;

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
use glutin_winit::{ApiPreference, DisplayBuilder, GlWindow};
#[cfg(not(target_arch = "wasm32"))]
use winit::raw_window_handle::HasWindowHandle;

#[cfg(not(target_arch = "wasm32"))]
use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{CursorGrabMode, Window, WindowAttributes},
};

#[cfg(not(target_arch = "wasm32"))]
use crate::gl_call;

#[derive(PartialEq, Copy, Clone)]
pub enum CursorMode {
    Normal,
    Hidden,
    #[allow(dead_code)]
    Disabled,
}

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(target_arch = "wasm32")]
pub type Platform = web_canvas::WebCanvasPlatform;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformBackend {
    NativeGlutin,
    WebCanvas,
}

pub struct RenderSurface<'a> {
    pub backend: PlatformBackend,
    pub fb_width: u32,
    pub fb_height: u32,
    pub scale_factor: f64,
    pub capabilities: &'a GlCapabilities,
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
            gl_version: "WebGL 2.0 (pending probe)".to_string(),
            glsl_version: "GLSL ES 3.00 (pending probe)".to_string(),
            vendor: String::new(),
            renderer: String::new(),
            extensions: Vec::new(),
            is_gles_like: true,
            supports_float_color_buffer: false,
            supports_msaa_float_renderbuffer: false,
            supports_clamp_to_border: false,
            supports_buffer_mapping: false,
            supports_instancing: true,
            supports_mrt: false,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(not(target_arch = "wasm32"))]
    pub fn log_startup_report(&self) {
        println!("GL capabilities:");
        println!("  Version: {}", self.gl_version);
        println!("  GLSL: {}", self.glsl_version);
        println!("  Vendor: {}", self.vendor);
        println!("  Renderer: {}", self.renderer);
        println!("  Extensions: {}", self.extensions.len());
        println!("  GLES/WebGL-like: {}", self.is_gles_like);
        println!("  Float color buffer: {}", self.supports_float_color_buffer);
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
    use std::{cell::RefCell, collections::HashMap, ffi::c_void};
    use js_sys::Int32Array;
    use wasm_bindgen::{JsCast, JsValue};
    use web_sys::{
        HtmlCanvasElement, WebGl2RenderingContext, WebGlBuffer, WebGlFramebuffer, WebGlProgram,
        WebGlRenderbuffer, WebGlShader, WebGlTexture, WebGlUniformLocation, WebGlVertexArrayObject,
    };

    thread_local! {
        static WEBGL_STATE: RefCell<Option<WebGlState>> = const { RefCell::new(None) };
    }

    struct WebGlState {
        context: WebGl2RenderingContext,
        next_id: u32,
        shaders: HashMap<u32, WebGlShader>,
        programs: HashMap<u32, WebGlProgram>,
        buffers: HashMap<u32, WebGlBuffer>,
        framebuffers: HashMap<u32, WebGlFramebuffer>,
        renderbuffers: HashMap<u32, WebGlRenderbuffer>,
        textures: HashMap<u32, WebGlTexture>,
        uniform_locations: HashMap<u32, WebGlUniformLocation>,
        vertex_arrays: HashMap<u32, WebGlVertexArrayObject>,
    }

    impl WebGlState {
        fn new(context: WebGl2RenderingContext) -> Self {
            Self {
                context,
                next_id: 1,
                shaders: HashMap::new(),
                programs: HashMap::new(),
                buffers: HashMap::new(),
                framebuffers: HashMap::new(),
                renderbuffers: HashMap::new(),
                textures: HashMap::new(),
                uniform_locations: HashMap::new(),
                vertex_arrays: HashMap::new(),
            }
        }

        fn next_handle(&mut self) -> u32 {
            let id = self.next_id;
            self.next_id += 1;
            id
        }
    }

    fn probe_hdr_framebuffers(ctx: &WebGl2RenderingContext) -> (bool, bool) {
        fn draw_buffers_gl(ctx: &WebGl2RenderingContext, attachments: &[u32]) {
            let array = js_sys::Array::new();
            for a in attachments {
                array.push(&JsValue::from_f64(*a as f64));
            }
            let v = JsValue::from(array);
            let _ = ctx.draw_buffers(&v);
        }

        const W: i32 = 4;
        const H: i32 = 4;

        let Some(fbo) = ctx.create_framebuffer() else {
            return (false, false);
        };
        let Some(tex0) = ctx.create_texture() else {
            ctx.delete_framebuffer(Some(&fbo));
            return (false, false);
        };
        let Some(depth) = ctx.create_texture() else {
            ctx.delete_texture(Some(&tex0));
            ctx.delete_framebuffer(Some(&fbo));
            return (false, false);
        };

        ctx.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&tex0));
        if ctx
            .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                WebGl2RenderingContext::TEXTURE_2D,
                0,
                WebGl2RenderingContext::RGBA16F as i32,
                W,
                H,
                0,
                WebGl2RenderingContext::RGBA,
                WebGl2RenderingContext::HALF_FLOAT,
                None,
            )
            .is_err()
        {
            ctx.bind_texture(WebGl2RenderingContext::TEXTURE_2D, None);
            ctx.delete_texture(Some(&tex0));
            ctx.delete_texture(Some(&depth));
            ctx.delete_framebuffer(Some(&fbo));
            return (false, false);
        }
        ctx.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MIN_FILTER,
            WebGl2RenderingContext::LINEAR as i32,
        );
        ctx.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MAG_FILTER,
            WebGl2RenderingContext::LINEAR as i32,
        );
        ctx.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_WRAP_S,
            WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
        );
        ctx.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_WRAP_T,
            WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
        );

        ctx.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&depth));
        if ctx
            .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                WebGl2RenderingContext::TEXTURE_2D,
                0,
                WebGl2RenderingContext::DEPTH_COMPONENT24 as i32,
                W,
                H,
                0,
                WebGl2RenderingContext::DEPTH_COMPONENT,
                WebGl2RenderingContext::UNSIGNED_INT,
                None,
            )
            .is_err()
        {
            ctx.bind_texture(WebGl2RenderingContext::TEXTURE_2D, None);
            ctx.delete_texture(Some(&tex0));
            ctx.delete_texture(Some(&depth));
            ctx.delete_framebuffer(Some(&fbo));
            return (false, false);
        }
        ctx.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MIN_FILTER,
            WebGl2RenderingContext::NEAREST as i32,
        );
        ctx.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MAG_FILTER,
            WebGl2RenderingContext::NEAREST as i32,
        );
        ctx.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_WRAP_S,
            WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
        );
        ctx.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_WRAP_T,
            WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
        );

        ctx.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, Some(&fbo));
        ctx.framebuffer_texture_2d(
            WebGl2RenderingContext::FRAMEBUFFER,
            WebGl2RenderingContext::COLOR_ATTACHMENT0,
            WebGl2RenderingContext::TEXTURE_2D,
            Some(&tex0),
            0,
        );
        ctx.framebuffer_texture_2d(
            WebGl2RenderingContext::FRAMEBUFFER,
            WebGl2RenderingContext::DEPTH_ATTACHMENT,
            WebGl2RenderingContext::TEXTURE_2D,
            Some(&depth),
            0,
        );
        draw_buffers_gl(ctx, &[WebGl2RenderingContext::COLOR_ATTACHMENT0]);

        let status = ctx.check_framebuffer_status(WebGl2RenderingContext::FRAMEBUFFER);
        let float_ok = status == WebGl2RenderingContext::FRAMEBUFFER_COMPLETE;

        let mut mrt_ok = false;
        if float_ok {
            if let Some(tex1) = ctx.create_texture() {
                ctx.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&tex1));
                let tex1_ok = ctx
                    .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                        WebGl2RenderingContext::TEXTURE_2D,
                        0,
                        WebGl2RenderingContext::RGBA16F as i32,
                        W,
                        H,
                        0,
                        WebGl2RenderingContext::RGBA,
                        WebGl2RenderingContext::HALF_FLOAT,
                        None,
                    )
                    .is_ok();
                if tex1_ok {
                    ctx.tex_parameteri(
                        WebGl2RenderingContext::TEXTURE_2D,
                        WebGl2RenderingContext::TEXTURE_MIN_FILTER,
                        WebGl2RenderingContext::LINEAR as i32,
                    );
                    ctx.tex_parameteri(
                        WebGl2RenderingContext::TEXTURE_2D,
                        WebGl2RenderingContext::TEXTURE_MAG_FILTER,
                        WebGl2RenderingContext::LINEAR as i32,
                    );
                    ctx.tex_parameteri(
                        WebGl2RenderingContext::TEXTURE_2D,
                        WebGl2RenderingContext::TEXTURE_WRAP_S,
                        WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
                    );
                    ctx.tex_parameteri(
                        WebGl2RenderingContext::TEXTURE_2D,
                        WebGl2RenderingContext::TEXTURE_WRAP_T,
                        WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
                    );
                    ctx.framebuffer_texture_2d(
                        WebGl2RenderingContext::FRAMEBUFFER,
                        WebGl2RenderingContext::COLOR_ATTACHMENT1,
                        WebGl2RenderingContext::TEXTURE_2D,
                        Some(&tex1),
                        0,
                    );
                    draw_buffers_gl(
                        ctx,
                        &[
                            WebGl2RenderingContext::COLOR_ATTACHMENT0,
                            WebGl2RenderingContext::COLOR_ATTACHMENT1,
                        ],
                    );
                    mrt_ok = ctx.check_framebuffer_status(WebGl2RenderingContext::FRAMEBUFFER)
                        == WebGl2RenderingContext::FRAMEBUFFER_COMPLETE;
                }
                ctx.delete_texture(Some(&tex1));
            }
        }

        ctx.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
        ctx.bind_texture(WebGl2RenderingContext::TEXTURE_2D, None);
        ctx.delete_texture(Some(&tex0));
        ctx.delete_texture(Some(&depth));
        ctx.delete_framebuffer(Some(&fbo));

        (float_ok, mrt_ok)
    }

    fn probe_webgl_capabilities(ctx: &WebGl2RenderingContext) -> GlCapabilities {
        let gl_version = ctx
            .get_parameter(WebGl2RenderingContext::VERSION)
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "WebGL 2.0".to_string());
        let glsl_version = ctx
            .get_parameter(WebGl2RenderingContext::SHADING_LANGUAGE_VERSION)
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "GLSL ES 3.00".to_string());
        let vendor = ctx
            .get_parameter(WebGl2RenderingContext::VENDOR)
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        let renderer = ctx
            .get_parameter(WebGl2RenderingContext::RENDERER)
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();

        let mut extensions: Vec<String> = Vec::new();
        if let Some(arr) = ctx.get_supported_extensions() {
            for i in 0..arr.length() {
                if let Some(name) = arr.get(i).as_string() {
                    extensions.push(name);
                }
            }
        }

        let has_ext = |name: &str| extensions.iter().any(|e| e == name);

        for ext_name in ["EXT_color_buffer_half_float", "EXT_color_buffer_float"] {
            let _ = ctx.get_extension(ext_name);
        }

        let max_color_attachments = ctx
            .get_parameter(WebGl2RenderingContext::MAX_COLOR_ATTACHMENTS)
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as i32;
        let max_draw_buffers = ctx
            .get_parameter(WebGl2RenderingContext::MAX_DRAW_BUFFERS)
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as i32;

        let supports_clamp_to_border =
            has_ext("EXT_texture_border_clamp") || has_ext("OES_texture_border_clamp");

        let (float_probe_ok, mrt_probe_ok) = probe_hdr_framebuffers(ctx);
        let supports_float_color_buffer = float_probe_ok;
        let supports_mrt = mrt_probe_ok && max_color_attachments >= 2 && max_draw_buffers >= 2;

        GlCapabilities {
            gl_version,
            glsl_version,
            vendor,
            renderer,
            extensions,
            is_gles_like: true,
            supports_float_color_buffer,
            supports_msaa_float_renderbuffer: false,
            supports_clamp_to_border,
            supports_buffer_mapping: false,
            supports_instancing: true,
            supports_mrt,
        }
    }

    #[allow(dead_code)]
    pub struct WebCanvasPlatform {
        pub canvas: HtmlCanvasElement,
        pub context: WebGl2RenderingContext,
        pub capabilities: GlCapabilities,
        pub fb_width: u32,
        pub fb_height: u32,
        pub scale_factor: f64,
        pub cursor_mode: CursorMode,
    }

    /// Physical backing-store size so the WebGL buffer matches CSS layout × `devicePixelRatio`.
    fn web_canvas_buffer_dimensions(
        canvas: &HtmlCanvasElement,
        window: &web_sys::Window,
        fallback_css_w: u32,
        fallback_css_h: u32,
    ) -> (u32, u32) {
        let dpr = window.device_pixel_ratio();
        let css_w = canvas.client_width();
        let css_h = canvas.client_height();
        let (bw, bh) = if css_w > 0 && css_h > 0 {
            (
                ((f64::from(css_w) * dpr).round() as u32).max(1),
                ((f64::from(css_h) * dpr).round() as u32).max(1),
            )
        } else {
            (
                (((fallback_css_w as f64) * dpr).round() as u32).max(1),
                (((fallback_css_h as f64) * dpr).round() as u32).max(1),
            )
        };
        (bw, bh)
    }

    impl WebCanvasPlatform {
        #[allow(dead_code)]
        pub fn backend(&self) -> PlatformBackend {
            PlatformBackend::WebCanvas
        }

        /// Sizes the canvas backing store from layout + DPR. Call when the window resizes or DPR changes.
        /// Returns `true` if width/height (or DPR) changed so GPU resources should be resized.
        pub fn sync_canvas_buffer_to_display(
            &mut self,
            fallback_css_w: u32,
            fallback_css_h: u32,
        ) -> bool {
            let window = match web_sys::window() {
                Some(w) => w,
                None => return false,
            };
            let dpr = window.device_pixel_ratio();
            let (bw, bh) = web_canvas_buffer_dimensions(
                &self.canvas,
                &window,
                fallback_css_w,
                fallback_css_h,
            );
            if bw == self.fb_width && bh == self.fb_height && (self.scale_factor - dpr).abs() < 1e-6 {
                return false;
            }
            self.canvas.set_width(bw);
            self.canvas.set_height(bh);
            self.fb_width = bw;
            self.fb_height = bh;
            self.scale_factor = dpr;
            true
        }

        /// Mouse events use CSS pixel coordinates; picking and rays use framebuffer pixels.
        pub fn canvas_css_to_framebuffer_px(&self, x_css: f32, y_css: f32) -> glam::Vec2 {
            let cw = self.canvas.client_width().max(1) as f32;
            let ch = self.canvas.client_height().max(1) as f32;
            glam::Vec2::new(
                x_css * self.fb_width as f32 / cw,
                y_css * self.fb_height as f32 / ch,
            )
        }

        pub fn render_surface(&self) -> super::RenderSurface<'_> {
            super::RenderSurface {
                backend: self.backend(),
                fb_width: self.fb_width,
                fb_height: self.fb_height,
                scale_factor: self.scale_factor,
                capabilities: &self.capabilities,
            }
        }

        #[allow(dead_code)]
        pub fn framebuffer_size(&self) -> (u32, u32) {
            (self.fb_width, self.fb_height)
        }

        pub fn new(canvas_id: &str, w: u32, h: u32) -> Result<Self, JsValue> {
            let window = web_sys::window().ok_or_else(|| JsValue::from_str("missing window"))?;
            let document = window
                .document()
                .ok_or_else(|| JsValue::from_str("missing document"))?;

            let canvas = match document.get_element_by_id(canvas_id) {
                Some(element) => element.dyn_into::<HtmlCanvasElement>()?,
                None => {
                    let canvas = document
                        .create_element("canvas")?
                        .dyn_into::<HtmlCanvasElement>()?;
                    canvas.set_id(canvas_id);
                    let body = document
                        .body()
                        .ok_or_else(|| JsValue::from_str("missing document body"))?;
                    body.append_child(&canvas)?;
                    canvas
                }
            };

            let window_ref = &window;
            let (buf_w, buf_h) = web_canvas_buffer_dimensions(&canvas, window_ref, w, h);
            canvas.set_width(buf_w);
            canvas.set_height(buf_h);

            let context = canvas
                .get_context("webgl2")?
                .ok_or_else(|| JsValue::from_str("WebGL2 is not available"))?
                .dyn_into::<WebGl2RenderingContext>()?;

            Ok(Self {
                canvas,
                context,
                capabilities: GlCapabilities::webgl2_defaults(),
                fb_width: buf_w,
                fb_height: buf_h,
                scale_factor: window.device_pixel_ratio(),
                cursor_mode: CursorMode::Normal,
            })
        }

        pub fn load_gl(&mut self) {
            WEBGL_STATE.with(|state| {
                *state.borrow_mut() = Some(WebGlState::new(self.context.clone()));
            });

            gl::load_with(webgl_proc_address);
            self.capabilities = probe_webgl_capabilities(&self.context);
        }

        pub fn swap_buffers(&self) {}

        pub fn request_pointer_lock(&self) {
            self.canvas.request_pointer_lock();
        }

        pub fn set_cursor_mode(&mut self, mode: CursorMode) {
            if self.cursor_mode == mode {
                return;
            }

            match mode {
                CursorMode::Normal => {
                    let _ = self.canvas.style().set_property("cursor", "default");
                    self.canvas.owner_document().and_then(|document| {
                        document.exit_pointer_lock();
                        Some(())
                    });
                }
                CursorMode::Hidden | CursorMode::Disabled => {
                    let _ = self.canvas.style().set_property("cursor", "none");
                }
            }

            self.cursor_mode = mode;
        }

        #[allow(dead_code)]
        pub fn clear_placeholder(&self) {
            unsafe {
                gl::Viewport(0, 0, self.fb_width as i32, self.fb_height as i32);
                gl::ClearColor(0.07, 0.08, 0.11, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            }
        }

        #[allow(dead_code)]
        pub fn placeholder(w: u32, h: u32) -> Self {
            Self::new("game-canvas", w, h).expect("failed to create WebGL2 canvas")
        }
    }

    fn webgl_proc_address(symbol: &str) -> *const c_void {
        match symbol {
            "glAttachShader" | "AttachShader" => webgl_attach_shader as *const c_void,
            "glActiveTexture" | "ActiveTexture" => webgl_active_texture as *const c_void,
            "glBindBuffer" | "BindBuffer" => webgl_bind_buffer as *const c_void,
            "glBindRenderbuffer" | "BindRenderbuffer" => webgl_bind_renderbuffer as *const c_void,
            "glBindFramebuffer" | "BindFramebuffer" => webgl_bind_framebuffer as *const c_void,
            "glBindTexture" | "BindTexture" => webgl_bind_texture as *const c_void,
            "glBufferData" | "BufferData" => webgl_buffer_data as *const c_void,
            "glCheckFramebufferStatus" | "CheckFramebufferStatus" => {
                webgl_check_framebuffer_status as *const c_void
            }
            "glClear" | "Clear" => webgl_clear as *const c_void,
            "glClearColor" | "ClearColor" => webgl_clear_color as *const c_void,
            "glCompileShader" | "CompileShader" => webgl_compile_shader as *const c_void,
            "glCullFace" | "CullFace" => webgl_cull_face as *const c_void,
            "glCreateProgram" | "CreateProgram" => webgl_create_program as *const c_void,
            "glCreateShader" | "CreateShader" => webgl_create_shader as *const c_void,
            "glDeleteShader" | "DeleteShader" => webgl_delete_shader as *const c_void,
            "glDeleteFramebuffers" | "DeleteFramebuffers" => {
                webgl_delete_framebuffers as *const c_void
            }
            "glDeleteRenderbuffers" | "DeleteRenderbuffers" => {
                webgl_delete_renderbuffers as *const c_void
            }
            "glDeleteTextures" | "DeleteTextures" => webgl_delete_textures as *const c_void,
            "glDisable" | "Disable" => webgl_disable as *const c_void,
            "glDepthFunc" | "DepthFunc" => webgl_depth_func as *const c_void,
            "glDepthMask" | "DepthMask" => webgl_depth_mask as *const c_void,
            "glDrawArrays" | "DrawArrays" => webgl_draw_arrays as *const c_void,
            "glDrawBuffers" | "DrawBuffers" => webgl_draw_buffers as *const c_void,
            "glDrawElements" | "DrawElements" => webgl_draw_elements as *const c_void,
            "glDrawArraysInstanced" | "DrawArraysInstanced" => {
                webgl_draw_arrays_instanced as *const c_void
            }
            "glEnable" | "Enable" => webgl_enable as *const c_void,
            "glBlendFunc" | "BlendFunc" => webgl_blend_func as *const c_void,
            "glFramebufferTexture2D" | "FramebufferTexture2D" => {
                webgl_framebuffer_texture_2d as *const c_void
            }
            "glFramebufferRenderbuffer" | "FramebufferRenderbuffer" => {
                webgl_framebuffer_renderbuffer as *const c_void
            }
            "glFrontFace" | "FrontFace" => webgl_front_face as *const c_void,
            "glGetIntegerv" | "GetIntegerv" => webgl_get_integerv as *const c_void,
            "glGetShaderInfoLog" | "GetShaderInfoLog" => webgl_get_shader_info_log as *const c_void,
            "glGetShaderiv" | "GetShaderiv" => webgl_get_shaderiv as *const c_void,
            "glGetUniformLocation" | "GetUniformLocation" => {
                webgl_get_uniform_location as *const c_void
            }
            "glEnableVertexAttribArray" | "EnableVertexAttribArray" => {
                webgl_enable_vertex_attrib_array as *const c_void
            }
            "glGenBuffers" | "GenBuffers" => webgl_gen_buffers as *const c_void,
            "glGenFramebuffers" | "GenFramebuffers" => webgl_gen_framebuffers as *const c_void,
            "glGenTextures" | "GenTextures" => webgl_gen_textures as *const c_void,
            "glBindVertexArray" | "BindVertexArray" => webgl_bind_vertex_array as *const c_void,
            "glGenVertexArrays" | "GenVertexArrays" => webgl_gen_vertex_arrays as *const c_void,
            "glGenRenderbuffers" | "GenRenderbuffers" => webgl_gen_renderbuffers as *const c_void,
            "glGenerateMipmap" | "GenerateMipmap" => webgl_generate_mipmap as *const c_void,
            "glLinkProgram" | "LinkProgram" => webgl_link_program as *const c_void,
            "glPixelStorei" | "PixelStorei" => webgl_pixel_storei as *const c_void,
            "glReadBuffer" | "ReadBuffer" => webgl_read_buffer as *const c_void,
            "glRenderbufferStorage" | "RenderbufferStorage" => {
                webgl_renderbuffer_storage as *const c_void
            }
            "glScissor" | "Scissor" => webgl_scissor as *const c_void,
            "glShaderSource" | "ShaderSource" => webgl_shader_source as *const c_void,
            "glTexImage2D" | "TexImage2D" => webgl_tex_image_2d as *const c_void,
            "glTexSubImage2D" | "TexSubImage2D" => webgl_tex_sub_image_2d as *const c_void,
            "glTexParameterfv" | "TexParameterfv" => webgl_tex_parameterfv as *const c_void,
            "glTexParameteri" | "TexParameteri" => webgl_tex_parameteri as *const c_void,
            "glUniform1f" | "Uniform1f" => webgl_uniform_1f as *const c_void,
            "glUniform1i" | "Uniform1i" => webgl_uniform_1i as *const c_void,
            "glUniform2f" | "Uniform2f" => webgl_uniform_2f as *const c_void,
            "glUniform3f" | "Uniform3f" => webgl_uniform_3f as *const c_void,
            "glUniformMatrix4fv" | "UniformMatrix4fv" => webgl_uniform_matrix_4fv as *const c_void,
            "glUseProgram" | "UseProgram" => webgl_use_program as *const c_void,
            "glVertexAttribIPointer" | "VertexAttribIPointer" => {
                webgl_vertex_attrib_i_pointer as *const c_void
            }
            "glVertexAttribPointer" | "VertexAttribPointer" => {
                webgl_vertex_attrib_pointer as *const c_void
            }
            "glVertexAttribDivisor" | "VertexAttribDivisor" => {
                webgl_vertex_attrib_divisor as *const c_void
            }
            "glViewport" | "Viewport" => webgl_viewport as *const c_void,
            _ => std::ptr::null(),
        }
    }

    unsafe extern "system" fn webgl_clear(mask: gl::types::GLbitfield) {
        with_webgl_state(|state| state.context.clear(mask));
    }

    unsafe extern "system" fn webgl_clear_color(
        red: gl::types::GLfloat,
        green: gl::types::GLfloat,
        blue: gl::types::GLfloat,
        alpha: gl::types::GLfloat,
    ) {
        with_webgl_state(|state| state.context.clear_color(red, green, blue, alpha));
    }

    unsafe extern "system" fn webgl_viewport(
        x: gl::types::GLint,
        y: gl::types::GLint,
        width: gl::types::GLsizei,
        height: gl::types::GLsizei,
    ) {
        with_webgl_state(|state| state.context.viewport(x, y, width, height));
    }

    unsafe extern "system" fn webgl_scissor(
        x: gl::types::GLint,
        y: gl::types::GLint,
        width: gl::types::GLsizei,
        height: gl::types::GLsizei,
    ) {
        with_webgl_state(|state| state.context.scissor(x, y, width, height));
    }

    unsafe extern "system" fn webgl_disable(cap: gl::types::GLenum) {
        with_webgl_state(|state| state.context.disable(cap));
    }

    unsafe extern "system" fn webgl_enable(cap: gl::types::GLenum) {
        with_webgl_state(|state| state.context.enable(cap));
    }

    unsafe extern "system" fn webgl_pixel_storei(
        pname: gl::types::GLenum,
        param: gl::types::GLint,
    ) {
        with_webgl_state(|state| state.context.pixel_storei(pname, param));
    }

    unsafe extern "system" fn webgl_blend_func(
        sfactor: gl::types::GLenum,
        dfactor: gl::types::GLenum,
    ) {
        with_webgl_state(|state| state.context.blend_func(sfactor, dfactor));
    }

    unsafe extern "system" fn webgl_depth_func(func: gl::types::GLenum) {
        with_webgl_state(|state| state.context.depth_func(func));
    }

    unsafe extern "system" fn webgl_depth_mask(flag: gl::types::GLboolean) {
        with_webgl_state(|state| state.context.depth_mask(flag != gl::FALSE));
    }

    unsafe extern "system" fn webgl_cull_face(mode: gl::types::GLenum) {
        with_webgl_state(|state| state.context.cull_face(mode));
    }

    unsafe extern "system" fn webgl_front_face(mode: gl::types::GLenum) {
        with_webgl_state(|state| state.context.front_face(mode));
    }

    unsafe extern "system" fn webgl_draw_arrays(
        mode: gl::types::GLenum,
        first: gl::types::GLint,
        count: gl::types::GLsizei,
    ) {
        with_webgl_state(|state| state.context.draw_arrays(mode, first, count));
    }

    unsafe extern "system" fn webgl_draw_elements(
        mode: gl::types::GLenum,
        count: gl::types::GLsizei,
        element_type: gl::types::GLenum,
        indices: *const c_void,
    ) {
        with_webgl_state(|state| {
            state
                .context
                .draw_elements_with_i32(mode, count, element_type, indices as i32);
        });
    }

    unsafe extern "system" fn webgl_draw_arrays_instanced(
        mode: gl::types::GLenum,
        first: gl::types::GLint,
        count: gl::types::GLsizei,
        instance_count: gl::types::GLsizei,
    ) {
        with_webgl_state(|state| {
            state
                .context
                .draw_arrays_instanced(mode, first, count, instance_count);
        });
    }

    unsafe extern "system" fn webgl_gen_vertex_arrays(
        n: gl::types::GLsizei,
        arrays: *mut gl::types::GLuint,
    ) {
        if arrays.is_null() || n <= 0 {
            return;
        }

        with_webgl_state_mut(|state| {
            for index in 0..n {
                let id = if let Some(vertex_array) = state.context.create_vertex_array() {
                    let id = state.next_handle();
                    state.vertex_arrays.insert(id, vertex_array);
                    id
                } else {
                    0
                };

                *arrays.add(index as usize) = id;
            }
        });
    }

    unsafe extern "system" fn webgl_bind_vertex_array(array: gl::types::GLuint) {
        with_webgl_state(|state| {
            let vertex_array = state.vertex_arrays.get(&array);
            state.context.bind_vertex_array(vertex_array);
        });
    }

    unsafe extern "system" fn webgl_gen_buffers(
        n: gl::types::GLsizei,
        buffers: *mut gl::types::GLuint,
    ) {
        if buffers.is_null() || n <= 0 {
            return;
        }

        with_webgl_state_mut(|state| {
            for index in 0..n {
                let id = if let Some(buffer) = state.context.create_buffer() {
                    let id = state.next_handle();
                    state.buffers.insert(id, buffer);
                    id
                } else {
                    0
                };

                *buffers.add(index as usize) = id;
            }
        });
    }

    unsafe extern "system" fn webgl_gen_framebuffers(
        n: gl::types::GLsizei,
        framebuffers: *mut gl::types::GLuint,
    ) {
        if framebuffers.is_null() || n <= 0 {
            return;
        }

        with_webgl_state_mut(|state| {
            for index in 0..n {
                let id = if let Some(framebuffer) = state.context.create_framebuffer() {
                    let id = state.next_handle();
                    state.framebuffers.insert(id, framebuffer);
                    id
                } else {
                    0
                };

                *framebuffers.add(index as usize) = id;
            }
        });
    }

    unsafe extern "system" fn webgl_bind_framebuffer(
        target: gl::types::GLenum,
        framebuffer: gl::types::GLuint,
    ) {
        with_webgl_state(|state| {
            let framebuffer = state.framebuffers.get(&framebuffer);
            state.context.bind_framebuffer(target, framebuffer);
        });
    }

    unsafe extern "system" fn webgl_bind_renderbuffer(
        target: gl::types::GLenum,
        renderbuffer: gl::types::GLuint,
    ) {
        with_webgl_state(|state| {
            let rb = state.renderbuffers.get(&renderbuffer);
            state.context.bind_renderbuffer(target, rb);
        });
    }

    unsafe extern "system" fn webgl_gen_renderbuffers(
        n: gl::types::GLsizei,
        renderbuffers: *mut gl::types::GLuint,
    ) {
        if renderbuffers.is_null() || n <= 0 {
            return;
        }

        with_webgl_state_mut(|state| {
            for index in 0..n {
                let id = if let Some(rb) = state.context.create_renderbuffer() {
                    let id = state.next_handle();
                    state.renderbuffers.insert(id, rb);
                    id
                } else {
                    0
                };

                *renderbuffers.add(index as usize) = id;
            }
        });
    }

    unsafe extern "system" fn webgl_renderbuffer_storage(
        target: gl::types::GLenum,
        internalformat: gl::types::GLenum,
        width: gl::types::GLsizei,
        height: gl::types::GLsizei,
    ) {
        with_webgl_state(|state| {
            state
                .context
                .renderbuffer_storage(target, internalformat, width, height);
        });
    }

    unsafe extern "system" fn webgl_framebuffer_renderbuffer(
        target: gl::types::GLenum,
        attachment: gl::types::GLenum,
        renderbuffertarget: gl::types::GLenum,
        renderbuffer: gl::types::GLuint,
    ) {
        with_webgl_state(|state| {
            let rb = state.renderbuffers.get(&renderbuffer);
            state
                .context
                .framebuffer_renderbuffer(target, attachment, renderbuffertarget, rb);
        });
    }

    unsafe extern "system" fn webgl_delete_framebuffers(
        n: gl::types::GLsizei,
        framebuffers: *const gl::types::GLuint,
    ) {
        if framebuffers.is_null() || n <= 0 {
            return;
        }
        let ids = std::slice::from_raw_parts(framebuffers, n as usize);
        with_webgl_state_mut(|state| {
            for &id in ids {
                if let Some(fbo) = state.framebuffers.remove(&id) {
                    state.context.delete_framebuffer(Some(&fbo));
                }
            }
        });
    }

    unsafe extern "system" fn webgl_delete_renderbuffers(
        n: gl::types::GLsizei,
        renderbuffers: *const gl::types::GLuint,
    ) {
        if renderbuffers.is_null() || n <= 0 {
            return;
        }
        let ids = std::slice::from_raw_parts(renderbuffers, n as usize);
        with_webgl_state_mut(|state| {
            for &id in ids {
                if let Some(rb) = state.renderbuffers.remove(&id) {
                    state.context.delete_renderbuffer(Some(&rb));
                }
            }
        });
    }

    unsafe extern "system" fn webgl_delete_textures(
        n: gl::types::GLsizei,
        textures: *const gl::types::GLuint,
    ) {
        if textures.is_null() || n <= 0 {
            return;
        }
        let ids = std::slice::from_raw_parts(textures, n as usize);
        with_webgl_state_mut(|state| {
            for &id in ids {
                if let Some(tex) = state.textures.remove(&id) {
                    state.context.delete_texture(Some(&tex));
                }
            }
        });
    }

    unsafe extern "system" fn webgl_get_integerv(
        pname: gl::types::GLenum,
        params: *mut gl::types::GLint,
    ) {
        if params.is_null() {
            return;
        }

        if pname != gl::VIEWPORT {
            return;
        }

        with_webgl_state(|state| {
            if let Ok(js) = state.context.get_parameter(pname) {
                if let Some(arr) = js.dyn_ref::<Int32Array>() {
                    let n = arr.length().min(4);
                    for i in 0..n {
                        *params.add(i as usize) = arr.get_index(i);
                    }
                }
            }
        });
    }

    unsafe extern "system" fn webgl_framebuffer_texture_2d(
        target: gl::types::GLenum,
        attachment: gl::types::GLenum,
        textarget: gl::types::GLenum,
        texture: gl::types::GLuint,
        level: gl::types::GLint,
    ) {
        with_webgl_state(|state| {
            let texture = state.textures.get(&texture);
            state
                .context
                .framebuffer_texture_2d(target, attachment, textarget, texture, level);
        });
    }

    unsafe extern "system" fn webgl_check_framebuffer_status(
        target: gl::types::GLenum,
    ) -> gl::types::GLenum {
        with_webgl_state(|state| state.context.check_framebuffer_status(target))
    }

    unsafe extern "system" fn webgl_draw_buffers(
        n: gl::types::GLsizei,
        bufs: *const gl::types::GLenum,
    ) {
        if bufs.is_null() || n <= 0 {
            return;
        }

        let values = std::slice::from_raw_parts(bufs, n as usize);
        let array = js_sys::Array::new();
        for value in values {
            array.push(&JsValue::from_f64(*value as f64));
        }

        with_webgl_state(|state| state.context.draw_buffers(&array));
    }

    unsafe extern "system" fn webgl_read_buffer(src: gl::types::GLenum) {
        with_webgl_state(|state| state.context.read_buffer(src));
    }

    unsafe extern "system" fn webgl_bind_buffer(
        target: gl::types::GLenum,
        buffer: gl::types::GLuint,
    ) {
        with_webgl_state(|state| {
            let buffer = state.buffers.get(&buffer);
            state.context.bind_buffer(target, buffer);
        });
    }

    unsafe extern "system" fn webgl_buffer_data(
        target: gl::types::GLenum,
        size: gl::types::GLsizeiptr,
        data: *const c_void,
        usage: gl::types::GLenum,
    ) {
        if data.is_null() || size <= 0 {
            return;
        }

        let bytes = std::slice::from_raw_parts(data.cast::<u8>(), size as usize);
        let array = js_sys::Uint8Array::view(bytes);
        with_webgl_state(|state| {
            state
                .context
                .buffer_data_with_array_buffer_view(target, &array, usage);
        });
    }

    unsafe extern "system" fn webgl_enable_vertex_attrib_array(index: gl::types::GLuint) {
        with_webgl_state(|state| state.context.enable_vertex_attrib_array(index));
    }

    unsafe extern "system" fn webgl_vertex_attrib_pointer(
        index: gl::types::GLuint,
        size: gl::types::GLint,
        attrib_type: gl::types::GLenum,
        normalized: gl::types::GLboolean,
        stride: gl::types::GLsizei,
        pointer: *const c_void,
    ) {
        with_webgl_state(|state| {
            state.context.vertex_attrib_pointer_with_i32(
                index,
                size,
                attrib_type,
                normalized != gl::FALSE,
                stride,
                pointer as i32,
            );
        });
    }

    unsafe extern "system" fn webgl_vertex_attrib_i_pointer(
        index: gl::types::GLuint,
        size: gl::types::GLint,
        attrib_type: gl::types::GLenum,
        stride: gl::types::GLsizei,
        pointer: *const c_void,
    ) {
        with_webgl_state(|state| {
            state.context.vertex_attrib_i_pointer_with_i32(
                index,
                size,
                attrib_type,
                stride,
                pointer as i32,
            );
        });
    }

    unsafe extern "system" fn webgl_vertex_attrib_divisor(
        index: gl::types::GLuint,
        divisor: gl::types::GLuint,
    ) {
        with_webgl_state(|state| state.context.vertex_attrib_divisor(index, divisor));
    }

    unsafe extern "system" fn webgl_active_texture(texture: gl::types::GLenum) {
        with_webgl_state(|state| state.context.active_texture(texture));
    }

    unsafe extern "system" fn webgl_gen_textures(
        n: gl::types::GLsizei,
        textures: *mut gl::types::GLuint,
    ) {
        if textures.is_null() || n <= 0 {
            return;
        }

        with_webgl_state_mut(|state| {
            for index in 0..n {
                let id = if let Some(texture) = state.context.create_texture() {
                    let id = state.next_handle();
                    state.textures.insert(id, texture);
                    id
                } else {
                    0
                };

                *textures.add(index as usize) = id;
            }
        });
    }

    unsafe extern "system" fn webgl_bind_texture(
        target: gl::types::GLenum,
        texture: gl::types::GLuint,
    ) {
        with_webgl_state(|state| {
            let texture = state.textures.get(&texture);
            state.context.bind_texture(target, texture);
        });
    }

    unsafe extern "system" fn webgl_tex_parameteri(
        target: gl::types::GLenum,
        pname: gl::types::GLenum,
        param: gl::types::GLint,
    ) {
        with_webgl_state(|state| state.context.tex_parameteri(target, pname, param));
    }

    unsafe extern "system" fn webgl_tex_parameterfv(
        _target: gl::types::GLenum,
        _pname: gl::types::GLenum,
        params: *const gl::types::GLfloat,
    ) {
        if params.is_null() {
            return;
        }
    }

    unsafe extern "system" fn webgl_tex_image_2d(
        target: gl::types::GLenum,
        level: gl::types::GLint,
        internalformat: gl::types::GLint,
        width: gl::types::GLsizei,
        height: gl::types::GLsizei,
        border: gl::types::GLint,
        format: gl::types::GLenum,
        type_: gl::types::GLenum,
        pixels: *const c_void,
    ) {
        let byte_len = if pixels.is_null() || width <= 0 || height <= 0 {
            0
        } else {
            texture_byte_len(width, height, format, type_)
        };
        let data = if byte_len == 0 {
            None
        } else {
            Some(std::slice::from_raw_parts(pixels.cast::<u8>(), byte_len))
        };

        with_webgl_state(|state| {
            let result = state
                .context
                .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                    target,
                    level,
                    internalformat,
                    width,
                    height,
                    border,
                    format,
                    type_,
                    data,
                );

            if let Err(error) = result {
                web_sys::console::error_1(&error);
            }
        });
    }

    unsafe extern "system" fn webgl_tex_sub_image_2d(
        target: gl::types::GLenum,
        level: gl::types::GLint,
        xoffset: gl::types::GLint,
        yoffset: gl::types::GLint,
        width: gl::types::GLsizei,
        height: gl::types::GLsizei,
        format: gl::types::GLenum,
        type_: gl::types::GLenum,
        pixels: *const c_void,
    ) {
        with_webgl_state(|state| {
            // web-sys names overloads by parameter types; see `WebGl2RenderingContext::texSubImage2D`.
            let result: Result<(), JsValue> = if pixels.is_null() {
                // Pixel data from bound PIXEL_UNPACK_BUFFER at byte offset 0.
                state.context.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_i32(
                    target,
                    level,
                    xoffset,
                    yoffset,
                    width,
                    height,
                    format,
                    type_,
                    0,
                )
            } else if width <= 0 || height <= 0 {
                Ok(())
            } else {
                let byte_len = texture_byte_len(width, height, format, type_);
                let data = std::slice::from_raw_parts(pixels.cast::<u8>(), byte_len);
                state
                    .context
                    .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_u8_array(
                        target,
                        level,
                        xoffset,
                        yoffset,
                        width,
                        height,
                        format,
                        type_,
                        Some(data),
                    )
            };

            if let Err(error) = result {
                web_sys::console::error_1(&error);
            }
        });
    }

    unsafe extern "system" fn webgl_generate_mipmap(target: gl::types::GLenum) {
        with_webgl_state(|state| state.context.generate_mipmap(target));
    }

    unsafe extern "system" fn webgl_create_program() -> gl::types::GLuint {
        with_webgl_state_mut(|state| {
            let Some(program) = state.context.create_program() else {
                return 0;
            };
            let id = state.next_handle();
            state.programs.insert(id, program);
            id
        })
    }

    unsafe extern "system" fn webgl_create_shader(
        shader_type: gl::types::GLenum,
    ) -> gl::types::GLuint {
        with_webgl_state_mut(|state| {
            let Some(shader) = state.context.create_shader(shader_type) else {
                return 0;
            };
            let id = state.next_handle();
            state.shaders.insert(id, shader);
            id
        })
    }

    unsafe extern "system" fn webgl_shader_source(
        shader: gl::types::GLuint,
        count: gl::types::GLsizei,
        string: *const *const gl::types::GLchar,
        length: *const gl::types::GLint,
    ) {
        let source = shader_source_from_raw_parts(count, string, length);
        with_webgl_state(|state| {
            if let Some(shader) = state.shaders.get(&shader) {
                state.context.shader_source(shader, &source);
            }
        });
    }

    unsafe extern "system" fn webgl_compile_shader(shader: gl::types::GLuint) {
        with_webgl_state(|state| {
            if let Some(shader) = state.shaders.get(&shader) {
                state.context.compile_shader(shader);
            }
        });
    }

    unsafe extern "system" fn webgl_get_shaderiv(
        shader: gl::types::GLuint,
        pname: gl::types::GLenum,
        params: *mut gl::types::GLint,
    ) {
        if params.is_null() {
            return;
        }

        let value = with_webgl_state(|state| {
            let Some(shader) = state.shaders.get(&shader) else {
                return 0;
            };

            if pname == gl::COMPILE_STATUS {
                return state
                    .context
                    .get_shader_parameter(shader, WebGl2RenderingContext::COMPILE_STATUS)
                    .as_bool()
                    .map(i32::from)
                    .unwrap_or(0);
            }

            0
        });

        *params = value;
    }

    unsafe extern "system" fn webgl_get_shader_info_log(
        shader: gl::types::GLuint,
        max_length: gl::types::GLsizei,
        length: *mut gl::types::GLsizei,
        info_log: *mut gl::types::GLchar,
    ) {
        let log = with_webgl_state(|state| {
            state
                .shaders
                .get(&shader)
                .and_then(|shader| state.context.get_shader_info_log(shader))
                .unwrap_or_default()
        });

        write_gl_string(&log, max_length, length, info_log);
    }

    unsafe extern "system" fn webgl_attach_shader(
        program: gl::types::GLuint,
        shader: gl::types::GLuint,
    ) {
        with_webgl_state(|state| {
            if let (Some(program), Some(shader)) =
                (state.programs.get(&program), state.shaders.get(&shader))
            {
                state.context.attach_shader(program, shader);
            }
        });
    }

    unsafe extern "system" fn webgl_link_program(program: gl::types::GLuint) {
        with_webgl_state(|state| {
            if let Some(program) = state.programs.get(&program) {
                state.context.link_program(program);
            }
        });
    }

    unsafe extern "system" fn webgl_delete_shader(shader: gl::types::GLuint) {
        with_webgl_state_mut(|state| {
            if let Some(shader) = state.shaders.remove(&shader) {
                state.context.delete_shader(Some(&shader));
            }
        });
    }

    unsafe extern "system" fn webgl_use_program(program: gl::types::GLuint) {
        with_webgl_state(|state| {
            let program = state.programs.get(&program);
            state.context.use_program(program);
        });
    }

    unsafe extern "system" fn webgl_get_uniform_location(
        program: gl::types::GLuint,
        name: *const gl::types::GLchar,
    ) -> gl::types::GLint {
        if name.is_null() {
            return -1;
        }

        let name = std::ffi::CStr::from_ptr(name).to_string_lossy();
        with_webgl_state_mut(|state| {
            let Some(program) = state.programs.get(&program) else {
                return -1;
            };
            let Some(location) = state.context.get_uniform_location(program, &name) else {
                return -1;
            };

            let id = state.next_handle();
            state.uniform_locations.insert(id, location);
            id as i32
        })
    }

    unsafe extern "system" fn webgl_uniform_1f(location: gl::types::GLint, v0: gl::types::GLfloat) {
        with_webgl_state(|state| {
            let location = uniform_location(state, location);
            state.context.uniform1f(location, v0);
        });
    }

    unsafe extern "system" fn webgl_uniform_1i(location: gl::types::GLint, v0: gl::types::GLint) {
        with_webgl_state(|state| {
            let location = uniform_location(state, location);
            state.context.uniform1i(location, v0);
        });
    }

    unsafe extern "system" fn webgl_uniform_2f(
        location: gl::types::GLint,
        v0: gl::types::GLfloat,
        v1: gl::types::GLfloat,
    ) {
        with_webgl_state(|state| {
            let location = uniform_location(state, location);
            state.context.uniform2f(location, v0, v1);
        });
    }

    unsafe extern "system" fn webgl_uniform_3f(
        location: gl::types::GLint,
        v0: gl::types::GLfloat,
        v1: gl::types::GLfloat,
        v2: gl::types::GLfloat,
    ) {
        with_webgl_state(|state| {
            let location = uniform_location(state, location);
            state.context.uniform3f(location, v0, v1, v2);
        });
    }

    unsafe extern "system" fn webgl_uniform_matrix_4fv(
        location: gl::types::GLint,
        count: gl::types::GLsizei,
        transpose: gl::types::GLboolean,
        value: *const gl::types::GLfloat,
    ) {
        if value.is_null() || count <= 0 {
            return;
        }

        let values = std::slice::from_raw_parts(value, count as usize * 16);
        with_webgl_state(|state| {
            let location = uniform_location(state, location);
            state.context.uniform_matrix4fv_with_f32_array(
                location,
                transpose != gl::FALSE,
                values,
            );
        });
    }

    fn uniform_location(
        state: &WebGlState,
        location: gl::types::GLint,
    ) -> Option<&WebGlUniformLocation> {
        if location < 0 {
            None
        } else {
            state.uniform_locations.get(&(location as u32))
        }
    }

    fn texture_byte_len(
        width: gl::types::GLsizei,
        height: gl::types::GLsizei,
        format: gl::types::GLenum,
        type_: gl::types::GLenum,
    ) -> usize {
        let channels = match format {
            gl::RGBA => 4,
            gl::RGB => 3,
            gl::RED | gl::ALPHA | 0x1909 => 1,
            0x190A => 2,
            _ => 4,
        };
        let bytes_per_channel = match type_ {
            gl::UNSIGNED_SHORT | gl::SHORT => 2,
            gl::HALF_FLOAT => 2,
            gl::UNSIGNED_INT | gl::INT | gl::FLOAT => 4,
            _ => 1,
        };

        width as usize * height as usize * channels * bytes_per_channel
    }

    fn with_webgl_state<T>(callback: impl FnOnce(&WebGlState) -> T) -> T {
        WEBGL_STATE.with(|state| {
            if let Some(state) = state.borrow().as_ref() {
                callback(state)
            } else {
                panic!("WebGL context was not loaded before gl call");
            }
        })
    }

    /// Set [`WebGl2RenderingContext::pixel_storei`] for `UNPACK_ALIGNMENT` only.
    ///
    /// The `gl` crate's Wasm import for `glPixelStorei` can hit a JS signature mismatch; font/UI
    /// uploads need this path to stay on `web_sys` like the rest of the custom loader.
    pub(crate) fn pixel_store_unpack_alignment(param: i32) {
        with_webgl_state(|state| {
            state.context.pixel_storei(gl::UNPACK_ALIGNMENT, param);
        });
    }

    fn with_webgl_state_mut<T>(callback: impl FnOnce(&mut WebGlState) -> T) -> T {
        WEBGL_STATE.with(|state| {
            if let Some(state) = state.borrow_mut().as_mut() {
                callback(state)
            } else {
                panic!("WebGL context was not loaded before gl call");
            }
        })
    }

    unsafe fn shader_source_from_raw_parts(
        count: gl::types::GLsizei,
        string: *const *const gl::types::GLchar,
        length: *const gl::types::GLint,
    ) -> String {
        let mut source = String::new();

        for index in 0..count {
            let source_ptr = *string.add(index as usize);
            if source_ptr.is_null() {
                continue;
            }

            let segment = if length.is_null() {
                std::ffi::CStr::from_ptr(source_ptr).to_string_lossy()
            } else {
                let source_length = *length.add(index as usize);
                if source_length < 0 {
                    std::ffi::CStr::from_ptr(source_ptr).to_string_lossy()
                } else {
                    let bytes =
                        std::slice::from_raw_parts(source_ptr.cast::<u8>(), source_length as usize);
                    String::from_utf8_lossy(bytes)
                }
            };

            source.push_str(&segment);
        }

        source
    }

    unsafe fn write_gl_string(
        value: &str,
        max_length: gl::types::GLsizei,
        length: *mut gl::types::GLsizei,
        output: *mut gl::types::GLchar,
    ) {
        if output.is_null() || max_length <= 0 {
            if !length.is_null() {
                *length = 0;
            }
            return;
        }

        let bytes = value.as_bytes();
        let write_len = bytes.len().min(max_length.saturating_sub(1) as usize);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), output.cast::<u8>(), write_len);
        *output.add(write_len) = 0;

        if !length.is_null() {
            *length = write_len as gl::types::GLsizei;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Platform {
    pub fn backend(&self) -> PlatformBackend {
        PlatformBackend::NativeGlutin
    }

    pub fn render_surface(&self) -> RenderSurface<'_> {
        RenderSurface {
            backend: self.backend(),
            fb_width: self.fb_width,
            fb_height: self.fb_height,
            scale_factor: self.scale_factor,
            capabilities: &self.capabilities,
        }
    }

    pub fn framebuffer_size(&self) -> (u32, u32) {
        (self.fb_width, self.fb_height)
    }

    pub fn swap_buffers(&self) {
        self.surface
            .swap_buffers(&self.gl_context)
            .expect("swap_buffers failed");
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

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
fn gl_string(name: u32) -> String {
    unsafe {
        let value = gl::GetString(name);
        if value.is_null() {
            return "Unavailable".to_string();
        }

        CStr::from_ptr(value.cast()).to_string_lossy().into_owned()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn gl_extensions() -> Vec<String> {
    let extension_count = gl_integer(gl::NUM_EXTENSIONS);
    if extension_count > 0 {
        return (0..extension_count)
            .filter_map(|index| unsafe {
                let extension = gl::GetStringi(gl::EXTENSIONS, index as u32);
                if extension.is_null() {
                    None
                } else {
                    Some(
                        CStr::from_ptr(extension.cast())
                            .to_string_lossy()
                            .into_owned(),
                    )
                }
            })
            .collect();
    }

    gl_string(gl::EXTENSIONS)
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn gl_integer(name: u32) -> i32 {
    let mut value = 0;
    unsafe {
        gl::GetIntegerv(name, &mut value);
    }
    value
}
