#![allow(clippy::too_many_arguments)]
use std::{
    collections::HashMap,
    ffi::c_void,
    mem::{self, offset_of},
    ptr::{self, null_mut},
};

use gl::CULL_FACE;
use glam::{vec3, vec4, Mat4};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};

use crate::{
    animation::model::{Model, Texture, Vertex},
    camera::Camera,
    config::game_config::GameConfig,
    entity_manager::EntityManager,
    enums_types::{
        FboType, FxaaLevels, ShaderType, TextureProfile, TextureType, Transform, VaoType,
    },
    gl_call,
    lights::Lights,
    particles::ParticleSystem,
    physics::PhysicsState,
    platform::{GlCapabilities, Platform},
    shaders::Shader,
    sound::sound_manager::SoundManager,
    util::constants::{
        BASIC_QUAD_VERTICES, FACES_CUBEMAP, SHADOW_HEIGHT, SHADOW_WIDTH, SKYBOX_INDICES,
        SKYBOX_VERTICES, UNIT_CUBE_VERTICES,
    },
};

pub struct BloomMip {
    fbo: u32,
    tex: u32,
    w: i32,
    h: i32,
}

struct HdrFramebuffer {
    fbo: u32,
    color: u32,
    bright: u32,
    depth: u32,
}

struct BloomPingpongFramebuffers {
    fbos: [u32; 2],
    textures: [u32; 2],
}

pub struct DefaultTextures {
    pub white: u32,
    pub black: u32,
    pub opaque: u32,
}

#[derive(Clone, Copy, Debug)]
enum RenderTargetColorFormat {
    Rgba16F,
    Rgba8,
}

impl RenderTargetColorFormat {
    fn label(self) -> &'static str {
        match self {
            Self::Rgba16F => "RGBA16F",
            Self::Rgba8 => "RGBA8",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum RenderTargetDepthFormat {
    DepthComponent24,
}

impl RenderTargetDepthFormat {
    fn label(self) -> &'static str {
        match self {
            Self::DepthComponent24 => "DEPTH_COMPONENT24",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct RenderTargetPolicy {
    compatibility_mode: bool,
    color_format: RenderTargetColorFormat,
    depth_format: RenderTargetDepthFormat,
    hdr_enabled: bool,
    bloom_enabled: bool,
    msaa_enabled: bool,
    fxaa_enabled: bool,
    mrt_enabled: bool,
}

impl RenderTargetPolicy {
    fn for_capabilities(capabilities: &GlCapabilities, config: &GameConfig) -> Self {
        let compatibility_mode = config.webgl_compatibility_mode || capabilities.is_gles_like;

        if compatibility_mode {
            return Self {
                compatibility_mode,
                color_format: RenderTargetColorFormat::Rgba8,
                depth_format: RenderTargetDepthFormat::DepthComponent24,
                hdr_enabled: false,
                bloom_enabled: false,
                msaa_enabled: false,
                fxaa_enabled: false,
                mrt_enabled: false,
            };
        }

        Self {
            compatibility_mode,
            color_format: RenderTargetColorFormat::Rgba16F,
            depth_format: RenderTargetDepthFormat::DepthComponent24,
            hdr_enabled: true,
            bloom_enabled: true,
            msaa_enabled: config.msaa_level != 1,
            fxaa_enabled: config.msaa_level < 2 && config.fxaa_level != FxaaLevels::Off,
            mrt_enabled: true,
        }
    }

    fn log_startup(self) {
        println!("Render target policy:");
        println!("  Compatibility mode: {}", self.compatibility_mode);
        println!("  Color format: {}", self.color_format.label());
        println!("  Depth format: {}", self.depth_format.label());
        println!("  HDR: {}", self.hdr_enabled);
        println!("  Bloom: {}", self.bloom_enabled);
        println!("  MSAA: {}", self.msaa_enabled);
        println!("  FXAA: {}", self.fxaa_enabled);
        println!("  Multiple render targets: {}", self.mrt_enabled);
    }
}

#[derive(Clone, Copy)]
pub enum UiTextureFormat {
    Rgba,
    Rgba8,
    AlphaMask,
}

#[derive(Clone, Copy)]
pub enum UiTextureFilter {
    Nearest,
    Linear,
}

#[derive(Clone, Copy)]
pub enum UiTextureWrap {
    ClampToEdge,
    Repeat,
}

#[derive(Clone, Copy)]
pub struct UiTextureDescriptor {
    pub width: u32,
    pub height: u32,
    pub format: UiTextureFormat,
    pub min_filter: UiTextureFilter,
    pub mag_filter: UiTextureFilter,
    pub wrap_s: UiTextureWrap,
    pub wrap_t: UiTextureWrap,
}

#[derive(Clone, Copy)]
pub struct UiUploadBuffer {
    id: u32,
}

impl UiTextureDescriptor {
    pub fn rgba_linear_clamped(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: UiTextureFormat::Rgba,
            min_filter: UiTextureFilter::Linear,
            mag_filter: UiTextureFilter::Linear,
            wrap_s: UiTextureWrap::ClampToEdge,
            wrap_t: UiTextureWrap::ClampToEdge,
        }
    }

    pub fn rgba_nearest_clamped(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: UiTextureFormat::Rgba,
            min_filter: UiTextureFilter::Nearest,
            mag_filter: UiTextureFilter::Nearest,
            wrap_s: UiTextureWrap::ClampToEdge,
            wrap_t: UiTextureWrap::ClampToEdge,
        }
    }

    pub fn rgba8_linear_clamped(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: UiTextureFormat::Rgba8,
            min_filter: UiTextureFilter::Linear,
            mag_filter: UiTextureFilter::Linear,
            wrap_s: UiTextureWrap::ClampToEdge,
            wrap_t: UiTextureWrap::ClampToEdge,
        }
    }

    pub fn alpha_linear_clamped(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: UiTextureFormat::AlphaMask,
            min_filter: UiTextureFilter::Linear,
            mag_filter: UiTextureFilter::Linear,
            wrap_s: UiTextureWrap::ClampToEdge,
            wrap_t: UiTextureWrap::ClampToEdge,
        }
    }
}

pub struct Renderer {
    pub capabilities: GlCapabilities,
    pub shaders: HashMap<ShaderType, Shader>,
    pub vaos: HashMap<VaoType, u32>,
    pub fbos: HashMap<FboType, u32>,
    pub defaults: DefaultTextures,
    pub depth_map: u32,
    pub cubemap_texture: u32,

    pub shadow_debug: bool,
    pub render_gizmos: bool,

    pub hdr_color: u32,
    pub hdr_bright: u32,

    pub pingpong_fbos: [u32; 2],
    pub pingpong_tex: [u32; 2],

    // Introspection stuff for ui
    pub exposure: f32,
    pub do_hdr: bool,
    pub bloom_strength: f32,
    pub do_msaa: bool,
    pub do_fxaa: bool,

    // COMING SOON
    //pub fxaa_level: FxaaLevels,
    pub fxaa_fbo: u32,
    pub fxaa_tex: u32,

    pub bloom_mips: Vec<BloomMip>,

    pub hdr_depth: u32,
}

impl Renderer {
    fn framebuffer_status_label(status: u32) -> &'static str {
        match status {
            gl::FRAMEBUFFER_COMPLETE => "FRAMEBUFFER_COMPLETE",
            gl::FRAMEBUFFER_UNDEFINED => "FRAMEBUFFER_UNDEFINED",
            gl::FRAMEBUFFER_INCOMPLETE_ATTACHMENT => "FRAMEBUFFER_INCOMPLETE_ATTACHMENT",
            gl::FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT => {
                "FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT"
            }
            gl::FRAMEBUFFER_INCOMPLETE_DRAW_BUFFER => "FRAMEBUFFER_INCOMPLETE_DRAW_BUFFER",
            gl::FRAMEBUFFER_INCOMPLETE_READ_BUFFER => "FRAMEBUFFER_INCOMPLETE_READ_BUFFER",
            gl::FRAMEBUFFER_UNSUPPORTED => "FRAMEBUFFER_UNSUPPORTED",
            gl::FRAMEBUFFER_INCOMPLETE_MULTISAMPLE => "FRAMEBUFFER_INCOMPLETE_MULTISAMPLE",
            gl::FRAMEBUFFER_INCOMPLETE_LAYER_TARGETS => "FRAMEBUFFER_INCOMPLETE_LAYER_TARGETS",
            _ => "UNKNOWN_FRAMEBUFFER_STATUS",
        }
    }

    fn check_framebuffer_complete(label: &str, details: impl AsRef<str>) {
        unsafe {
            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            if status != gl::FRAMEBUFFER_COMPLETE {
                panic!(
                    "{} incomplete: {} status=0x{:x} ({})",
                    label,
                    details.as_ref(),
                    status,
                    Self::framebuffer_status_label(status)
                );
            }
        }
    }

    fn create_hdr_framebuffer(
        width: u32,
        height: u32,
        policy: RenderTargetPolicy,
    ) -> HdrFramebuffer {
        assert!(policy.hdr_enabled);
        assert!(policy.mrt_enabled);
        assert!(matches!(
            policy.color_format,
            RenderTargetColorFormat::Rgba16F
        ));
        assert!(matches!(
            policy.depth_format,
            RenderTargetDepthFormat::DepthComponent24
        ));

        let mut fbo = 0;
        let mut depth = 0;
        let mut color_buffers: [u32; 2] = [0, 0];

        unsafe {
            gl_call!(gl::GenFramebuffers(1, &mut fbo));
            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, fbo));

            gl_call!(gl::GenTextures(2, color_buffers.as_mut_ptr()));

            for i in 0..2 {
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, color_buffers[i]));
                gl_call!(gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA16F as i32,
                    width as i32,
                    height as i32,
                    0,
                    gl::RGBA,
                    gl::FLOAT,
                    std::ptr::null()
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_MIN_FILTER,
                    gl::LINEAR as i32
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_MAG_FILTER,
                    gl::LINEAR as i32
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_WRAP_S,
                    gl::CLAMP_TO_EDGE as i32
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_WRAP_T,
                    gl::CLAMP_TO_EDGE as i32
                ));

                gl_call!(gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0 + i as u32,
                    gl::TEXTURE_2D,
                    color_buffers[i],
                    0
                ));
            }

            let attachments = [gl::COLOR_ATTACHMENT0, gl::COLOR_ATTACHMENT1];
            gl_call!(gl::DrawBuffers(
                attachments.len() as i32,
                attachments.as_ptr()
            ));

            gl_call!(gl::GenTextures(1, &mut depth));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, depth));

            gl_call!(gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::DEPTH_COMPONENT24 as i32,
                width as i32,
                height as i32,
                0,
                gl::DEPTH_COMPONENT,
                gl::UNSIGNED_INT,
                std::ptr::null(),
            ));

            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                gl::NEAREST as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAG_FILTER,
                gl::NEAREST as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_EDGE as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_EDGE as i32
            ));

            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_COMPARE_MODE,
                gl::NONE as i32
            ));

            gl_call!(gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::DEPTH_ATTACHMENT,
                gl::TEXTURE_2D,
                depth,
                0
            ));

            gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));

            Self::check_framebuffer_complete(
                "HDR FBO",
                format!(
                    "size={}x{} color={} bright={} depth={}",
                    width,
                    height,
                    policy.color_format.label(),
                    policy.color_format.label(),
                    policy.depth_format.label()
                ),
            );
            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, 0));
        }

        HdrFramebuffer {
            fbo,
            color: color_buffers[0],
            bright: color_buffers[1],
            depth,
        }
    }

    fn create_bloom_pingpong_framebuffers(
        width: u32,
        height: u32,
        policy: RenderTargetPolicy,
    ) -> BloomPingpongFramebuffers {
        assert!(policy.bloom_enabled);
        assert!(matches!(
            policy.color_format,
            RenderTargetColorFormat::Rgba16F
        ));

        let mut fbos = [0u32; 2];
        let mut textures = [0u32; 2];

        unsafe {
            gl_call!(gl::GenFramebuffers(2, fbos.as_mut_ptr()));
            gl_call!(gl::GenTextures(2, textures.as_mut_ptr()));

            for i in 0..2 {
                gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, fbos[i]));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, textures[i]));

                gl_call!(gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA16F as i32,
                    width as i32,
                    height as i32,
                    0,
                    gl::RGBA,
                    gl::FLOAT,
                    std::ptr::null()
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_MIN_FILTER,
                    gl::LINEAR as i32
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_MAG_FILTER,
                    gl::LINEAR as i32
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_WRAP_S,
                    gl::CLAMP_TO_EDGE as i32
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_WRAP_T,
                    gl::CLAMP_TO_EDGE as i32
                ));

                gl_call!(gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0,
                    gl::TEXTURE_2D,
                    textures[i],
                    0
                ));

                Self::check_framebuffer_complete(
                    &format!("Pingpong FBO {}", i),
                    format!(
                        "size={}x{} color={} depth=none",
                        width,
                        height,
                        policy.color_format.label()
                    ),
                );
            }

            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, 0));
        }

        BloomPingpongFramebuffers { fbos, textures }
    }

    fn create_hdr_msaa_framebuffer(
        width: u32,
        height: u32,
        policy: RenderTargetPolicy,
        msaa_level: i32,
    ) -> Option<u32> {
        assert!([1, 2, 4, 8, 16].contains(&msaa_level)); // 1 is off
        println!("MSAA LEVEL CHOSEN: {}", msaa_level);

        if !policy.msaa_enabled {
            unsafe { gl_call!(gl::Disable(gl::MULTISAMPLE)) };
            return None;
        }

        assert!(policy.hdr_enabled);
        assert!(policy.mrt_enabled);
        assert!(matches!(
            policy.color_format,
            RenderTargetColorFormat::Rgba16F
        ));
        assert!(matches!(
            policy.depth_format,
            RenderTargetDepthFormat::DepthComponent24
        ));

        let mut fbo = 0;

        unsafe {
            gl_call!(gl::GenFramebuffers(1, &mut fbo));
            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, fbo));

            let mut color_rb_msaa: [u32; 2] = [0, 0];
            gl_call!(gl::GenRenderbuffers(2, color_rb_msaa.as_mut_ptr()));

            for i in 0..2 {
                gl_call!(gl::BindRenderbuffer(gl::RENDERBUFFER, color_rb_msaa[i]));
                gl_call!(gl::RenderbufferStorageMultisample(
                    gl::RENDERBUFFER,
                    msaa_level,
                    gl::RGBA16F,
                    width as i32,
                    height as i32,
                ));
                gl_call!(gl::FramebufferRenderbuffer(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0 + i as u32,
                    gl::RENDERBUFFER,
                    color_rb_msaa[i],
                ));
            }

            let mut rbo_depth_msaa = 0;
            gl_call!(gl::GenRenderbuffers(1, &mut rbo_depth_msaa));
            gl_call!(gl::BindRenderbuffer(gl::RENDERBUFFER, rbo_depth_msaa));
            gl_call!(gl::RenderbufferStorageMultisample(
                gl::RENDERBUFFER,
                msaa_level,
                gl::DEPTH_COMPONENT24,
                width as i32,
                height as i32,
            ));
            gl_call!(gl::FramebufferRenderbuffer(
                gl::FRAMEBUFFER,
                gl::DEPTH_ATTACHMENT,
                gl::RENDERBUFFER,
                rbo_depth_msaa,
            ));

            let attachments = [gl::COLOR_ATTACHMENT0, gl::COLOR_ATTACHMENT1];
            gl_call!(gl::DrawBuffers(
                attachments.len() as i32,
                attachments.as_ptr()
            ));

            Self::check_framebuffer_complete(
                "HDR MSAA FBO",
                format!(
                    "size={}x{} samples={} color={} depth={}",
                    width,
                    height,
                    msaa_level,
                    policy.color_format.label(),
                    policy.depth_format.label()
                ),
            );

            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, 0));
        }

        Some(fbo)
    }

    pub fn new(platform: &Platform, config: &GameConfig) -> Self {
        let render_target_policy =
            RenderTargetPolicy::for_capabilities(&platform.capabilities, config);
        render_target_policy.log_startup();

        if render_target_policy.compatibility_mode {
            return Self::new_webgl_compatibility(platform, config, render_target_policy);
        }

        let render_gizmos = config.render_gizmos;
        // =============================================================
        // Setup Shaders
        // =============================================================
        let mut shaders = HashMap::new();
        let mut vaos = HashMap::new();
        let mut fbos = HashMap::new();

        let skybox_shader = Shader::new("resources/shaders/skybox.glsl");
        let debug_light_shader = Shader::new("resources/shaders/point_light.glsl");
        let depth_shader = Shader::new("resources/shaders/depth_shader.glsl");
        let text_shader = Shader::new("resources/shaders/text.glsl");
        text_shader.activate();
        let loc = unsafe {
            gl::GetUniformLocation(text_shader.id, b"textTexture\0".as_ptr() as *const _)
        };
        unsafe {
            gl::Uniform1i(loc, 1);
        }

        // Static model
        let static_model_shader = Shader::new("resources/shaders/model/static_model.glsl");
        static_model_shader.activate();
        static_model_shader.set_int("material.Diffuse", 1);
        static_model_shader.set_int("material.Specular", 2);
        static_model_shader.set_int("material.Emissive", 3);
        static_model_shader.set_int("material.Opacity", 4);
        static_model_shader.set_int("shadow_map", 7);
        static_model_shader.set_int("skybox", 10);
        static_model_shader.set_bool(
            "shadow_border_fallback",
            !platform.capabilities.supports_clamp_to_border,
        );

        // Animated model shader
        let animated_model_shader = Shader::new("resources/shaders/model/animated_model.glsl");
        animated_model_shader.activate();
        animated_model_shader.set_int("material.Diffuse", 1);
        animated_model_shader.set_int("material.Specular", 2);
        animated_model_shader.set_int("material.Emissive", 3);
        animated_model_shader.set_int("material.Opacity", 4);
        animated_model_shader.set_int("shadow_map", 7);
        animated_model_shader.set_int("skybox", 10);
        animated_model_shader.set_bool(
            "shadow_border_fallback",
            !platform.capabilities.supports_clamp_to_border,
        );

        let gizmo_shader = Shader::new("resources/shaders/gizmo.glsl");
        let particle_shader = Shader::new("resources/shaders/particles.glsl");
        let hdr_shader = Shader::new("resources/shaders/hdr.glsl");
        let blur_shader = Shader::new("resources/shaders/blur.glsl");
        let fxaa_shader = Shader::new("resources/shaders/fxaa.glsl");
        let bloom_down_shader = Shader::new("resources/shaders/bloom/bloom_downsample.glsl");
        let bloom_up_shader = Shader::new("resources/shaders/bloom/bloom_upsample.glsl");

        let mut vao = 0;
        let mut vbo = 0;
        let mut ebo = 0;
        let mut cubemap_texture = 0;

        // =============================================================
        // Main framebuffer (hdr end-result after multisampling)
        // =============================================================
        // we are using a custom framebuffer that is a floating point
        // buffer and that allows HDR

        // TODO: Dynamic resizing of the FBO
        let width = platform.fb_width;
        let height = platform.fb_height;

        let hdr_framebuffer = Self::create_hdr_framebuffer(width, height, render_target_policy);
        fbos.insert(FboType::HDR, hdr_framebuffer.fbo);
        let hdr_color = hdr_framebuffer.color;
        let hdr_bright = hdr_framebuffer.bright;
        let hdr_depth = hdr_framebuffer.depth;
        // =============================================================
        // Pingpong FBOs for bloom
        // =============================================================

        let bloom_pingpong =
            Self::create_bloom_pingpong_framebuffers(width, height, render_target_policy);
        let pingpong_fbos = bloom_pingpong.fbos;
        let pingpong_tex = bloom_pingpong.textures;

        // =============================================================
        // Multi sample framebuffer
        // =============================================================
        let hdr_msaa_fbo = Self::create_hdr_msaa_framebuffer(
            width,
            height,
            render_target_policy,
            config.msaa_level,
        );
        let do_msaa = hdr_msaa_fbo.is_some();
        if let Some(hdr_msaa_fbo) = hdr_msaa_fbo {
            fbos.insert(FboType::HdrMsaa, hdr_msaa_fbo);
        }

        // =============================================================
        // Skybox memes
        // =============================================================
        unsafe {
            skybox_shader.activate();
            gl_call!(gl::GenVertexArrays(1, &mut vao));
            gl_call!(gl::GenBuffers(1, &mut vbo));
            gl_call!(gl::GenBuffers(1, &mut ebo));

            vaos.insert(VaoType::Skybox, vao);

            println!("vao skybox: {}", vao);

            gl_call!(gl::BindVertexArray(vao));

            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, vbo));
            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                (mem::size_of::<f32>() * SKYBOX_VERTICES.len()) as isize,
                SKYBOX_VERTICES.as_ptr().cast(),
                gl::STATIC_DRAW
            ));

            gl_call!(gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo));
            gl_call!(gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (mem::size_of::<u32>() * SKYBOX_INDICES.len()) as isize,
                SKYBOX_INDICES.as_ptr().cast(),
                gl::STATIC_DRAW
            ));

            gl_call!(gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                (3 * mem::size_of::<f32>()) as i32,
                std::ptr::null(),
            ));
            gl_call!(gl::EnableVertexAttribArray(0));

            gl_call!(gl::BindVertexArray(0));
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, 0));
            gl_call!(gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0));

            // SKYBOX TEXTURES
            gl_call!(gl::GenTextures(1, &mut cubemap_texture));
            gl_call!(gl::BindTexture(gl::TEXTURE_CUBE_MAP, cubemap_texture));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_CUBE_MAP,
                gl::TEXTURE_MAG_FILTER,
                gl::LINEAR as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_CUBE_MAP,
                gl::TEXTURE_MIN_FILTER,
                gl::LINEAR as i32
            ));
            // These are very important to prevent seams
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_CUBE_MAP,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_EDGE as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_CUBE_MAP,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_EDGE as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_CUBE_MAP,
                gl::TEXTURE_WRAP_R,
                gl::CLAMP_TO_EDGE as i32
            ));

            for i in 0..FACES_CUBEMAP.len() {
                let img = match image::open(FACES_CUBEMAP[i]) {
                    Ok(img) => img,
                    _ => panic!("Error opening {}", FACES_CUBEMAP[i]),
                };
                let (img_width, img_height) = img.dimensions();
                let rgba = img.to_rgb8();
                let raw = rgba.as_raw();

                gl_call!(gl::TexImage2D(
                    gl::TEXTURE_CUBE_MAP_POSITIVE_X + i as u32,
                    0,
                    gl::SRGB8 as i32,
                    img_width as i32,
                    img_height as i32,
                    0,
                    gl::RGB,
                    gl::UNSIGNED_BYTE,
                    raw.as_ptr().cast()
                ));
            }
        }

        // =============================================================
        // Debug point light setup
        // =============================================================
        unsafe {
            debug_light_shader.activate();

            gl_call!(gl::GenVertexArrays(1, &mut vao));
            gl_call!(gl::GenBuffers(1, &mut vbo));

            vaos.insert(VaoType::DebugLight, vao);

            gl_call!(gl::BindVertexArray(vao));

            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, vbo));
            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                (mem::size_of::<f32>() * UNIT_CUBE_VERTICES.len()) as isize,
                UNIT_CUBE_VERTICES.as_ptr().cast(),
                gl::STATIC_DRAW
            ));

            // Position
            gl_call!(gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                8 * mem::size_of::<f32>() as i32,
                std::ptr::null(),
            ));
            gl_call!(gl::EnableVertexAttribArray(0));

            // Normal
            gl_call!(gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                8 * mem::size_of::<f32>() as i32,
                (5 * mem::size_of::<f32>()) as *const c_void
            ));
            gl_call!(gl::EnableVertexAttribArray(1));
        }

        // =============================================================
        // Shadow Mapping
        // =============================================================
        // The general idea is that we need to create a depth map rendered
        // from the perspective of the light source. In this case one
        // directional light.
        // We can do this using a "framebuffer". We have been using a
        // framebuffer all along, just the "default" one given to us.
        let mut fbo = 0;
        let mut depth_map = 0;
        unsafe {
            gl_call!(gl::GenFramebuffers(1, &mut fbo));

            fbos.insert(FboType::DepthMap, fbo);

            gl_call!(gl::GenTextures(1, &mut depth_map));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, depth_map));
            gl_call!(gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::DEPTH_COMPONENT as i32,
                SHADOW_WIDTH,
                SHADOW_HEIGHT,
                0,
                gl::DEPTH_COMPONENT,
                gl::FLOAT,
                null_mut()
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                gl::NEAREST as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAG_FILTER,
                gl::NEAREST as i32
            ));
            if platform.capabilities.supports_clamp_to_border {
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_WRAP_S,
                    gl::CLAMP_TO_BORDER as i32
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_WRAP_T,
                    gl::CLAMP_TO_BORDER as i32
                ));
                gl_call!(gl::TexParameterfv(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_BORDER_COLOR,
                    [1.0, 1.0, 1.0, 1.0].as_ptr().cast()
                ));
            } else {
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_WRAP_S,
                    gl::CLAMP_TO_EDGE as i32
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_WRAP_T,
                    gl::CLAMP_TO_EDGE as i32
                ));
            }

            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, fbo));
            gl_call!(gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::DEPTH_ATTACHMENT,
                gl::TEXTURE_2D,
                depth_map,
                0
            ));
            gl_call!(gl::DrawBuffer(gl::NONE));
            gl_call!(gl::ReadBuffer(gl::NONE));
            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, 0));
        }

        // =============================================================
        // Base Quad for Frames
        // =============================================================
        let mut quad_vao = 0;
        let mut quad_vbo = 0;

        unsafe {
            let quad_vertices: [f32; 30] = BASIC_QUAD_VERTICES;

            gl_call!(gl::GenVertexArrays(1, &mut quad_vao));
            gl_call!(gl::GenBuffers(1, &mut quad_vbo));

            gl_call!(gl::BindVertexArray(quad_vao));

            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, quad_vbo));
            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                (quad_vertices.len() * std::mem::size_of::<f32>()) as isize,
                quad_vertices.as_ptr().cast(),
                gl::STATIC_DRAW
            ));

            let stride = (5 * std::mem::size_of::<f32>()) as i32;

            // location 0: vec3 position
            gl_call!(gl::EnableVertexAttribArray(0));
            gl_call!(gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                stride,
                std::ptr::null()
            ));

            // location 1: vec2 uv (offset 3 floats)
            gl_call!(gl::EnableVertexAttribArray(1));
            gl_call!(gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (3 * std::mem::size_of::<f32>()) as *const _
            ));

            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, 0));
            gl_call!(gl::BindVertexArray(0));

            // TODO: HashMap lookup every quad draw is unnecessary overhead
            // Not huge, but we call render_quad() a lot (bloom passes).
            // storing this as just fields int a struct would be better.
            // e.g. pub struct Vaos { base_quad, hdr, etc }

            vaos.insert(VaoType::BaseQuad, quad_vao);
        }

        // =============================================================
        // Post-color texture for FXAA
        // =============================================================
        let mut fxaa_fbo = 0u32;
        let mut fxaa_tex = 0u32;

        unsafe {
            gl_call!(gl::GenFramebuffers(1, &mut fxaa_fbo));
            gl_call!(gl::GenTextures(1, &mut fxaa_tex));

            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, fxaa_fbo));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, fxaa_tex));

            gl_call!(gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as i32,
                width as i32,
                height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                std::ptr::null()
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                gl::LINEAR as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAG_FILTER,
                gl::LINEAR as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_EDGE as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_EDGE as i32
            ));

            gl_call!(gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                fxaa_tex,
                0
            ));

            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            if status != gl::FRAMEBUFFER_COMPLETE {
                panic!("Pingpong FBO {} incomplete: 0x{:x}", fxaa_fbo, status);
            }

            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, 0));
        }

        let mut debug_depth_quad = Shader::new("resources/shaders/debug_depth_quad.glsl");

        debug_depth_quad.activate();
        debug_depth_quad.store_uniform_location("depth_map");
        debug_depth_quad.set_int("depth_map", 0);
        //shaders.insert(ShaderType::Model, model_shader);
        shaders.insert(ShaderType::Skybox, skybox_shader);
        shaders.insert(ShaderType::DebugLight, debug_light_shader);
        shaders.insert(ShaderType::Depth, depth_shader);
        shaders.insert(ShaderType::DebugShadowMap, debug_depth_quad);
        shaders.insert(ShaderType::Text, text_shader);
        shaders.insert(ShaderType::Gizmo, gizmo_shader);
        shaders.insert(ShaderType::Particles, particle_shader);
        shaders.insert(ShaderType::HDR, hdr_shader);
        shaders.insert(ShaderType::Blur, blur_shader);
        shaders.insert(ShaderType::StaticModel, static_model_shader);
        shaders.insert(ShaderType::AnimatedModel, animated_model_shader);
        shaders.insert(ShaderType::Fxaa, fxaa_shader);
        shaders.insert(ShaderType::BloomDownsample, bloom_down_shader);
        shaders.insert(ShaderType::BloomUpsample, bloom_up_shader);

        // DEFAULT TEXTURES
        let defaults = DefaultTextures {
            white: Self::make_solid_texture(255, 255, 255, 255),
            black: Self::make_solid_texture(0, 0, 0, 255),
            opaque: Self::make_solid_texture(255, 255, 255, 255),
        };

        let mut renderer = Self {
            capabilities: platform.capabilities.clone(),
            shaders,
            vaos,
            fbos,
            depth_map,
            defaults,

            cubemap_texture,
            shadow_debug: false,
            render_gizmos,

            hdr_color,
            hdr_bright,

            pingpong_fbos,
            pingpong_tex,

            exposure: 1.5,
            do_hdr: render_target_policy.hdr_enabled,
            bloom_strength: if render_target_policy.bloom_enabled {
                0.1
            } else {
                0.0
            },
            do_msaa,
            do_fxaa: render_target_policy.fxaa_enabled,
            //fxaa_level: config.fxaa_level.clone(),
            fxaa_fbo,
            fxaa_tex,

            bloom_mips: vec![],
            hdr_depth,
        };

        renderer.create_bloom_chain(width, height);
        renderer
    }

    fn new_webgl_compatibility(
        platform: &Platform,
        config: &GameConfig,
        render_target_policy: RenderTargetPolicy,
    ) -> Self {
        let defaults = DefaultTextures {
            white: Self::make_solid_texture(255, 255, 255, 255),
            black: Self::make_solid_texture(0, 0, 0, 255),
            opaque: Self::make_solid_texture(255, 255, 255, 255),
        };

        Self {
            capabilities: platform.capabilities.clone(),
            shaders: HashMap::new(),
            vaos: HashMap::new(),
            fbos: HashMap::new(),
            defaults,
            depth_map: 0,
            cubemap_texture: 0,
            shadow_debug: false,
            render_gizmos: config.render_gizmos,
            hdr_color: 0,
            hdr_bright: 0,
            pingpong_fbos: [0; 2],
            pingpong_tex: [0; 2],
            exposure: 1.0,
            do_hdr: render_target_policy.hdr_enabled,
            bloom_strength: if render_target_policy.bloom_enabled {
                0.1
            } else {
                0.0
            },
            do_msaa: render_target_policy.msaa_enabled,
            do_fxaa: render_target_policy.fxaa_enabled,
            fxaa_fbo: 0,
            fxaa_tex: 0,
            bloom_mips: Vec::new(),
            hdr_depth: 0,
        }
    }

    pub fn render_webgl_compatibility_frame(&mut self, fb_width: u32, fb_height: u32) {
        unsafe {
            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, 0));
            gl_call!(gl::Viewport(0, 0, fb_width as i32, fb_height as i32));
            gl_call!(gl::Disable(gl::DEPTH_TEST));
            gl_call!(gl::Disable(CULL_FACE));
            gl_call!(gl::Disable(gl::SCISSOR_TEST));
            gl_call!(gl::ClearColor(0.02, 0.02, 0.03, 1.0));
            gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT));
        }
    }

    pub fn render_world(
        &mut self,
        em: &mut EntityManager,
        camera: &mut Camera,
        light_manager: &Lights,
        sound_manager: &mut SoundManager,
        fb_width: u32,
        fb_height: u32,
        elapsed: f32,
        ps: &PhysicsState,
        alpha: f32,
        particles: &mut ParticleSystem,
    ) {
        self.draw(
            em,
            camera,
            light_manager,
            sound_manager,
            fb_width,
            fb_height,
            elapsed,
            ps,
            alpha,
            particles,
        );
    }

    pub fn draw(
        &mut self,
        em: &mut EntityManager,
        camera: &mut Camera,
        light_manager: &Lights,
        sound_manager: &mut SoundManager,
        fb_width: u32,
        fb_height: u32,
        _elapsed: f32, // TODO: This is for the flashing white
        ps: &PhysicsState,
        alpha: f32,
        particles: &mut ParticleSystem,
    ) {
        // =============================================================
        // ANIMATION LOD
        // =============================================================
        // lod works by just seeing if there are more than 300 guys. If so, we skip animation
        // frames based on distance from the camera. No fustrum stuff, just simple radius calc
        let ids_by_type = em.get_ids_by_type();
        let cam_pos = camera.position;

        let mut visible_by_type: HashMap<String, Vec<usize>> = HashMap::new();

        const NEAR2: f32 = 15.0 * 15.0;
        const MID2: f32 = 35.0 * 35.0;
        const FAR2: f32 = 50.0 * 50.0;
        const FARTHEST2: f32 = 75.0 * 75.0;

        for (ty, ids) in ids_by_type.iter() {
            let mut out = Vec::with_capacity(ids.len());
            let enough_guys = ids.len() >= 300;

            for &id in ids.iter() {
                let t = match em.transforms.get(id) {
                    Some(t) => t,
                    None => continue,
                };

                let d2 = (t.position - cam_pos).length_squared();

                if let Some(animator) = em.animators.get_mut(id) {
                    if let Some(anim) = animator.animations.get_mut(&animator.current_animation) {
                        anim.lod_skip = if d2 < NEAR2 {
                            0
                        } else if d2 < MID2 && enough_guys {
                            1
                        } else if d2 < FAR2 && enough_guys {
                            3
                        } else if d2 < FARTHEST2 && enough_guys {
                            5
                        } else if enough_guys {
                            7
                        } else {
                            0
                        };
                    }
                }

                out.push(id);
            }

            if !out.is_empty() {
                visible_by_type.insert(ty.to_string(), out);
            }
        }

        // =============================================================
        // SHADOW PASS
        // =============================================================
        self.shadow_begin(camera, light_manager);

        for (_, ids) in ids_by_type.iter() {
            if ids.is_empty() {
                continue;
            }

            let is_animated = em.animators.contains(*ids.first().unwrap());
            self.shadow_draw_bucket(em, ps, alpha, ids, is_animated);
        }

        self.shadow_end();

        if self.shadow_debug {
            return;
        }

        // =============================================================
        // HDR FRAMEBUFFER
        // =============================================================
        // Render to the MSAA one if MSAA > 1, else use the regular
        unsafe {
            let scene_target = match self.do_msaa {
                true => *self.fbos.get(&FboType::HdrMsaa).unwrap(),
                false => *self.fbos.get(&FboType::HDR).unwrap(),
            };

            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, scene_target));
            let attachments = [gl::COLOR_ATTACHMENT0, gl::COLOR_ATTACHMENT1];
            gl_call!(gl::DrawBuffers(2, attachments.as_ptr()));
            gl_call!(gl::Viewport(0, 0, fb_width as i32, fb_height as i32));
            gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));
        }

        self.skybox_pass(camera, fb_width, fb_height);
        if self.render_gizmos {
            let gizmo_ids = em.get_gizmo_ids();
            self.gizmo_pass(camera, em, gizmo_ids, ps, alpha);
        }

        for (_, ids) in ids_by_type.iter() {
            if ids.len() > 0 {
                let is_animated = em.animators.contains(*ids.first().unwrap());

                self.model_pass(
                    camera,
                    em,
                    light_manager,
                    ps,
                    alpha,
                    particles,
                    sound_manager,
                    &ids,
                    is_animated,
                );
            }
        }

        particles.render(
            self.shaders.get_mut(&ShaderType::Particles).unwrap(),
            camera,
        );

        // ==========================================================
        // blitting MSAA: read MSAA and draw texture FBO
        // ==========================================================
        if self.do_msaa {
            assert!(!self.do_fxaa);
            unsafe {
                let hdr_msaa_fbo = *self.fbos.get(&FboType::HdrMsaa).unwrap();
                let hdr_fbo = *self.fbos.get(&FboType::HDR).unwrap();

                // bind for blit
                gl_call!(gl::BindFramebuffer(gl::READ_FRAMEBUFFER, hdr_msaa_fbo));
                gl_call!(gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, hdr_fbo));

                gl_call!(gl::ReadBuffer(gl::COLOR_ATTACHMENT0));
                gl_call!(gl::DrawBuffer(gl::COLOR_ATTACHMENT0));
                gl_call!(gl::BlitFramebuffer(
                    0,
                    0,
                    fb_width as i32,
                    fb_height as i32,
                    0,
                    0,
                    fb_width as i32,
                    fb_height as i32,
                    gl::COLOR_BUFFER_BIT,
                    gl::NEAREST,
                ));

                gl_call!(gl::ReadBuffer(gl::COLOR_ATTACHMENT1));
                gl_call!(gl::DrawBuffer(gl::COLOR_ATTACHMENT1));
                gl_call!(gl::BlitFramebuffer(
                    0,
                    0,
                    fb_width as i32,
                    fb_height as i32,
                    0,
                    0,
                    fb_width as i32,
                    fb_height as i32,
                    gl::COLOR_BUFFER_BIT,
                    gl::NEAREST,
                ));

                gl_call!(gl::BlitFramebuffer(
                    0,
                    0,
                    fb_width as i32,
                    fb_height as i32,
                    0,
                    0,
                    fb_width as i32,
                    fb_height as i32,
                    gl::DEPTH_BUFFER_BIT,
                    gl::NEAREST
                ));
            }
        }

        let blurred_bloom_tex = self.bloom_down_up(fb_width, fb_height);

        // ==========================================================
        // HDR COMBINE (and do fxaa if enabled)
        // ==========================================================

        let hdr_out_fbo = if !self.do_msaa && self.do_fxaa {
            self.fxaa_fbo
        } else {
            0
        };

        unsafe {
            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, hdr_out_fbo));
            gl_call!(gl::Viewport(0, 0, fb_width as i32, fb_height as i32));
            gl_call!(gl::Disable(gl::DEPTH_TEST));
            gl_call!(gl::Clear(
                gl::COLOR_BUFFER_BIT /* |  gl::DEPTH_BUFFER_BIT */
            ));
        }

        let hdr_shader = self.shaders.get_mut(&ShaderType::HDR).unwrap();
        hdr_shader.activate();
        hdr_shader.set_int("hdrBuffer", 0);
        hdr_shader.set_int("bloomBuffer", 1);
        hdr_shader.set_float("exposure", self.exposure);
        hdr_shader.set_bool("hdr", self.do_hdr);
        hdr_shader.set_float("bloomStrength", self.bloom_strength);
        hdr_shader.set_int("uDepth", 2);
        hdr_shader.set_mat4("uInvProj", camera.projection.inverse());

        unsafe {
            gl_call!(gl::ActiveTexture(gl::TEXTURE0));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, self.hdr_color));

            gl_call!(gl::ActiveTexture(gl::TEXTURE1));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, blurred_bloom_tex));

            gl_call!(gl::ActiveTexture(gl::TEXTURE2));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, self.hdr_depth));
        }

        self.render_quad();

        // ==========================================================
        // FXAA PASS
        // ==========================================================
        if !self.do_msaa && self.do_fxaa {
            unsafe {
                gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, 0));
                gl_call!(gl::Viewport(0, 0, fb_width as i32, fb_height as i32));
                gl_call!(gl::Disable(gl::DEPTH_TEST));
                gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT));
            }

            let fxaa_shader = self.shaders.get_mut(&ShaderType::Fxaa).unwrap();
            fxaa_shader.activate();
            fxaa_shader.set_int("uColor", 0);
            fxaa_shader.set_vec2(
                "uInvResolution",
                1.0 / fb_width as f32,
                1.0 / fb_height as f32,
            );

            unsafe {
                gl_call!(gl::ActiveTexture(gl::TEXTURE0));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, self.fxaa_tex));
            }

            self.render_quad();
        }

        unsafe {
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
        }
    }

    fn gizmo_pass(
        &mut self,
        camera: &mut Camera,
        em: &EntityManager,
        ids: Vec<usize>,
        _ps: &PhysicsState,
        alpha: f32,
    ) {
        if !self.supports_gizmo_wireframe() {
            return;
        }

        unsafe {
            gl_call!(gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE));
        }

        let shader = self.shaders.get_mut(&ShaderType::Gizmo).unwrap();
        shader.activate();
        for id in ids {
            let model = match em.collider_gizmos.get(id) {
                Some(model) => model,
                None => continue,
            };

            let curr = em.collider_transforms.get(id).unwrap();
            let prev = em.prev_collider_transforms.get(id).unwrap();

            let trans = Self::render_transform_from_args(em, curr, prev, alpha);

            let m_mat =
                Mat4::from_scale_rotation_translation(trans.scale, trans.rotation, trans.position);

            shader.set_mat4("model", m_mat);
            shader.set_mat4("projection", camera.projection);
            shader.set_mat4("view", camera.view);
            Self::draw_model(model, shader);
        }

        unsafe {
            gl_call!(gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL));
        }
    }

    fn supports_gizmo_wireframe(&self) -> bool {
        !self.capabilities.is_gles_like
    }

    fn make_solid_texture(r: u8, g: u8, b: u8, a: u8) -> u32 {
        let mut id = 0;
        unsafe {
            gl::GenTextures(1, &mut id);
            gl::BindTexture(gl::TEXTURE_2D, id);
            let pix = [r, g, b, a];
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as i32,
                1,
                1,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                pix.as_ptr() as *const _,
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
        id
    }

    fn ui_texture_internal_format(format: UiTextureFormat) -> i32 {
        match format {
            UiTextureFormat::Rgba => gl::RGBA as i32,
            UiTextureFormat::Rgba8 => gl::RGBA8 as i32,
            UiTextureFormat::AlphaMask => gl::R8 as i32,
        }
    }

    fn ui_texture_upload_format(format: UiTextureFormat) -> u32 {
        match format {
            UiTextureFormat::Rgba => gl::RGBA,
            UiTextureFormat::Rgba8 => gl::RGBA,
            UiTextureFormat::AlphaMask => gl::RED,
        }
    }

    fn ui_texture_filter(filter: UiTextureFilter) -> i32 {
        match filter {
            UiTextureFilter::Nearest => gl::NEAREST as i32,
            UiTextureFilter::Linear => gl::LINEAR as i32,
        }
    }

    fn ui_texture_wrap(wrap: UiTextureWrap) -> i32 {
        match wrap {
            UiTextureWrap::ClampToEdge => gl::CLAMP_TO_EDGE as i32,
            UiTextureWrap::Repeat => gl::REPEAT as i32,
        }
    }

    fn with_ui_unpack_alignment(format: UiTextureFormat, f: impl FnOnce()) {
        unsafe {
            if matches!(format, UiTextureFormat::AlphaMask) {
                gl_call!(gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1));
            }
            f();
            if matches!(format, UiTextureFormat::AlphaMask) {
                gl_call!(gl::PixelStorei(gl::UNPACK_ALIGNMENT, 4));
            }
        }
    }

    pub fn create_ui_texture(desc: UiTextureDescriptor, pixels: Option<&[u8]>) -> u32 {
        let mut texture = 0u32;
        unsafe {
            gl_call!(gl::GenTextures(1, &mut texture));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, texture));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                Self::ui_texture_filter(desc.min_filter)
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAG_FILTER,
                Self::ui_texture_filter(desc.mag_filter)
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_S,
                Self::ui_texture_wrap(desc.wrap_s)
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_T,
                Self::ui_texture_wrap(desc.wrap_t)
            ));
        }
        Self::resize_ui_texture_with_pixels(texture, desc, pixels);
        texture
    }

    pub fn resize_ui_texture(texture: u32, desc: UiTextureDescriptor) {
        Self::resize_ui_texture_with_pixels(texture, desc, None);
    }

    fn resize_ui_texture_with_pixels(
        texture: u32,
        desc: UiTextureDescriptor,
        pixels: Option<&[u8]>,
    ) {
        let data = pixels.map_or(ptr::null(), |p| p.as_ptr().cast());
        Self::with_ui_unpack_alignment(desc.format, || unsafe {
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, texture));
            gl_call!(gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                Self::ui_texture_internal_format(desc.format),
                desc.width as i32,
                desc.height as i32,
                0,
                Self::ui_texture_upload_format(desc.format),
                gl::UNSIGNED_BYTE,
                data
            ));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
        });
    }

    pub fn update_ui_texture_from_upload_buffer(
        texture: u32,
        upload_buffer: UiUploadBuffer,
        desc: UiTextureDescriptor,
    ) {
        Self::with_ui_unpack_alignment(desc.format, || unsafe {
            gl_call!(gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, upload_buffer.id));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, texture));
            gl_call!(gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                desc.width as i32,
                desc.height as i32,
                Self::ui_texture_upload_format(desc.format),
                gl::UNSIGNED_BYTE,
                ptr::null()
            ));
            gl_call!(gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, 0));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
        });
    }

    pub fn update_ui_texture_from_pixels(
        texture: u32,
        desc: UiTextureDescriptor,
        pixels: &[u8],
    ) {
        Self::update_ui_texture_region(
            texture,
            desc.format,
            0,
            0,
            desc.width as i32,
            desc.height as i32,
            pixels,
        );
    }

    pub fn update_ui_texture_region(
        texture: u32,
        format: UiTextureFormat,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        pixels: &[u8],
    ) {
        Self::with_ui_unpack_alignment(format, || unsafe {
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, texture));
            gl_call!(gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                x,
                y,
                width,
                height,
                Self::ui_texture_upload_format(format),
                gl::UNSIGNED_BYTE,
                pixels.as_ptr().cast()
            ));
        });
    }

    pub fn create_ui_upload_buffer() -> UiUploadBuffer {
        let mut id = 0;
        unsafe {
            gl_call!(gl::GenBuffers(1, &mut id));
        }
        UiUploadBuffer { id }
    }

    pub fn delete_ui_upload_buffer(upload_buffer: UiUploadBuffer) {
        unsafe {
            gl_call!(gl::DeleteBuffers(1, &upload_buffer.id));
        }
    }

    pub fn write_ui_upload_buffer<T>(
        upload_buffer: UiUploadBuffer,
        byte_len: isize,
        item_count: usize,
        allocate_storage: bool,
        write: impl FnOnce(&mut [T]),
    ) {
        unsafe {
            gl_call!(gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, upload_buffer.id));
            if allocate_storage {
                gl_call!(gl::BufferData(
                    gl::PIXEL_UNPACK_BUFFER,
                    byte_len,
                    ptr::null(),
                    gl::STREAM_DRAW
                ));
            }

            let ptr = gl::MapBuffer(gl::PIXEL_UNPACK_BUFFER, gl::WRITE_ONLY);
            if !ptr.is_null() {
                let buffer_slice = std::slice::from_raw_parts_mut(ptr.cast::<T>(), item_count);
                write(buffer_slice);
                gl_call!(gl::UnmapBuffer(gl::PIXEL_UNPACK_BUFFER));
            }
        }
    }

    pub fn upload_model_mesh(model: &mut Model) {
        unsafe {
            gl_call!(gl::GenVertexArrays(1, &mut model.vao));
            gl_call!(gl::GenBuffers(1, &mut model.vbo));
            gl_call!(gl::GenBuffers(1, &mut model.ebo));

            gl_call!(gl::BindVertexArray(model.vao));
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, model.vbo));

            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                (mem::size_of::<Vertex>() * model.vertices.len()) as isize,
                model.vertices.as_ptr().cast(),
                gl::STATIC_DRAW,
            ));

            gl_call!(gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, model.ebo));
            gl_call!(gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (mem::size_of::<u32>() * model.indices.len()) as isize,
                model.indices.as_ptr().cast(),
                gl::STATIC_DRAW
            ));

            gl_call!(gl::EnableVertexAttribArray(0));
            gl_call!(gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                ptr::null(),
            ));

            gl_call!(gl::EnableVertexAttribArray(1));
            gl_call!(gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, normal) as *const _
            ));

            gl_call!(gl::EnableVertexAttribArray(2));
            gl_call!(gl::VertexAttribPointer(
                2,
                2,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, uv) as *const _
            ));

            gl_call!(gl::EnableVertexAttribArray(3));
            gl_call!(gl::VertexAttribPointer(
                3,
                4,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, base_color) as *const _
            ));

            gl_call!(gl::EnableVertexAttribArray(4));
            gl_call!(gl::VertexAttribIPointer(
                4,
                4,
                gl::INT,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, bone_ids) as *const _
            ));

            gl_call!(gl::EnableVertexAttribArray(5));
            gl_call!(gl::VertexAttribPointer(
                5,
                4,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, bone_weights) as *const _
            ));

            gl_call!(gl::BindVertexArray(0));
        }
    }

    pub fn upload_model_texture(
        model: &mut Model,
        path: String,
        texture_type: TextureType,
        texture_prof: TextureProfile,
    ) {
        println!("texture is {}", &path);
        let file_name = model.directory.clone() + "/" + path.as_str();

        dbg!(&path);
        dbg!(&file_name);

        let mut texture_id = 0;
        unsafe {
            gl_call!(gl::GenTextures(1, &mut texture_id));

            let img = match image::open(file_name.clone()) {
                Ok(data) => Some(data),
                Err(_) => {
                    if texture_type == TextureType::Diffuse {
                        // TODO: Parse BSDF color instead or something.
                        let mut imgbuf = ImageBuffer::new(1, 1);
                        let color_u8 = [198, 198, 198, 255];

                        for pixel in imgbuf.pixels_mut() {
                            *pixel = Rgba(color_u8);
                        }

                        let color_path = format!(
                            "{:.3}-{:.3}-{:.3}.png",
                            color_u8[0], color_u8[1], color_u8[2]
                        );
                        let save_loc = format!("{}/{}", model.directory, color_path);

                        imgbuf.save(save_loc).expect("Failed to save texture image");

                        Some(DynamicImage::ImageRgba8(imgbuf))
                    } else {
                        None
                    }
                }
            };

            if let Some(img) = img {
                let (img_width, img_height) = img.dimensions();
                let rgba = img.to_rgba8();
                let raw = rgba.as_raw();

                gl_call!(gl::BindTexture(gl::TEXTURE_2D, texture_id));
                gl_call!(gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::SRGB8 as i32,
                    img_width as i32,
                    img_height as i32,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    raw.as_ptr() as *const c_void
                ));

                match texture_prof {
                    TextureProfile::DecalCrisp => {
                        gl_call!(gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_WRAP_S,
                            gl::CLAMP_TO_EDGE as i32
                        ));
                        gl_call!(gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_WRAP_T,
                            gl::CLAMP_TO_EDGE as i32
                        ));
                        gl_call!(gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_MIN_FILTER,
                            gl::NEAREST as i32
                        ));
                        gl_call!(gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_MAG_FILTER,
                            gl::NEAREST as i32
                        ));
                    }
                    TextureProfile::BroadDefault => {
                        gl_call!(gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_WRAP_S,
                            gl::REPEAT as i32
                        ));
                        gl_call!(gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_WRAP_T,
                            gl::REPEAT as i32
                        ));
                        gl_call!(gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_MIN_FILTER,
                            gl::NEAREST_MIPMAP_LINEAR as i32
                        ));
                        gl_call!(gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_MAG_FILTER,
                            gl::NEAREST as i32
                        ));
                        gl_call!(gl::GenerateMipmap(gl::TEXTURE_2D));
                    }
                    TextureProfile::AlphaMasked => {
                        gl_call!(gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_WRAP_S,
                            gl::CLAMP_TO_EDGE as i32
                        ));
                        gl_call!(gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_WRAP_T,
                            gl::CLAMP_TO_EDGE as i32
                        ));
                        gl_call!(gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_MIN_FILTER,
                            gl::LINEAR_MIPMAP_LINEAR as i32
                        ));
                        gl_call!(gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_MAG_FILTER,
                            gl::LINEAR as i32
                        ));
                        gl_call!(gl::GenerateMipmap(gl::TEXTURE_2D));
                    }
                }

                let texture = Texture {
                    id: texture_id,
                    _type: texture_type.clone().to_string(),
                    path: file_name,
                };

                match texture_type {
                    TextureType::Diffuse => {
                        model.textures[1] = Some(texture);
                    }
                    TextureType::Specular => {
                        model.textures[2] = Some(texture);
                    }
                    TextureType::Emissive => {
                        model.textures[3] = Some(texture);
                    }
                    TextureType::NormalMap => {
                        model.textures[4] = Some(texture);
                    }
                    TextureType::Roughness => {
                        model.textures[5] = Some(texture);
                    }
                    TextureType::Metalness => {
                        model.textures[6] = Some(texture);
                    }
                    TextureType::Displacement => {
                        model.textures[7] = Some(texture);
                    }
                    TextureType::Opacity => {
                        model.textures[8] = Some(texture);
                    }
                }
            }
        }
    }

    pub fn draw_model(model: &Model, shader: &mut Shader) {
        if model.color_for_texture {
            shader.set_bool("use_base_color", true);
            shader.set_bool("has_opacity_texture", false);
        } else {
            shader.set_bool("use_base_color", false);
            if let Some(diff) = model.get_tex(1) {
                unsafe {
                    gl_call!(gl::ActiveTexture(gl::TEXTURE1));
                    gl_call!(gl::BindTexture(gl::TEXTURE_2D, diff.id));
                }
            }
            if let Some(spec) = model.get_tex(2) {
                unsafe {
                    gl_call!(gl::ActiveTexture(gl::TEXTURE2));
                    gl_call!(gl::BindTexture(gl::TEXTURE_2D, spec.id));
                }
            }
            if let Some(emis) = model.get_tex(3) {
                unsafe {
                    gl_call!(gl::ActiveTexture(gl::TEXTURE3));
                    gl_call!(gl::BindTexture(gl::TEXTURE_2D, emis.id));
                }
            }
            if let Some(opac) = model.get_tex(8) {
                shader.set_bool("has_opacity_texture", true);
                unsafe {
                    gl_call!(gl::ActiveTexture(gl::TEXTURE4));
                    gl_call!(gl::BindTexture(gl::TEXTURE_2D, opac.id));
                }
            } else {
                shader.set_bool("has_opacity_texture", false);
            }
        }

        Self::draw_model_geometry(model);

        shader.set_bool("has_opacity_texture", false);
        shader.set_bool("use_base_color", false);
    }

    pub fn draw_model_geometry(model: &Model) {
        unsafe {
            gl_call!(gl::BindVertexArray(model.vao));
            gl_call!(gl::DrawElements(
                gl::TRIANGLES,
                model.indices.len() as i32,
                gl::UNSIGNED_INT,
                ptr::null(),
            ));

            gl_call!(gl::BindVertexArray(0));
        }
    }

    fn model_pass(
        &mut self,
        camera: &mut Camera,
        em: &EntityManager,
        light_manager: &Lights,
        _ps: &PhysicsState,
        alpha: f32,
        particles: &mut ParticleSystem,
        sound_manager: &mut SoundManager,
        ids: &Vec<usize>,
        is_animated: bool,
    ) {
        unsafe {
            gl_call!(gl::Enable(gl::DEPTH_TEST));
            gl_call!(gl::DepthMask(gl::TRUE));
            gl_call!(gl::Disable(gl::BLEND));
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
            gl::FrontFace(gl::CCW);

            // Set default textures for models that don't have one
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, self.defaults.white); // Diffuse default
            gl::ActiveTexture(gl::TEXTURE2);
            gl::BindTexture(gl::TEXTURE_2D, self.defaults.black); // Spec default
            gl::ActiveTexture(gl::TEXTURE3);
            gl::BindTexture(gl::TEXTURE_2D, self.defaults.black); // Emissive default
            gl::ActiveTexture(gl::TEXTURE4);
            gl::BindTexture(gl::TEXTURE_2D, self.defaults.opaque); // Opacity default

            gl::ActiveTexture(gl::TEXTURE7);
            gl::BindTexture(gl::TEXTURE_2D, self.depth_map);
            gl::ActiveTexture(gl::TEXTURE10);
            gl::BindTexture(gl::TEXTURE_CUBE_MAP, self.cubemap_texture);
        }

        let shader = match is_animated {
            true => self.shaders.get_mut(&ShaderType::AnimatedModel).unwrap(),
            false => self.shaders.get_mut(&ShaderType::StaticModel).unwrap(),
        };

        shader.activate();

        for &is_alpha_pass in &[false, true] {
            shader.set_bool("alpha_test_pass", is_alpha_pass);

            // Hoist stuff
            shader.set_mat4("projection", camera.projection);
            shader.set_mat4("view", camera.view);
            shader.set_mat4("light_space_mat", camera.light_space);
            shader.set_dir_light("dir_light", &light_manager.dir_light);
            shader.set_float("bias_scalar", light_manager.bias_scalar);
            shader.set_vec3("view_position", camera.position);
            shader.set_int("skybox", 10);

            for id in ids.iter() {
                let is_selected = em.selected.contains(&id);
                if em.is_equipped.get(*id).is_none() && em.owners.get(*id).is_some() {
                    continue;
                }
                shader.set_bool("selection_fresnel", is_selected);

                let model = match em.models.get(*id) {
                    Some(m) => m,
                    None => continue,
                };
                let trans = Self::render_transform(em, *id, alpha);

                let m_mat = Mat4::from_scale_rotation_translation(
                    trans.scale,
                    trans.rotation,
                    trans.position,
                );
                shader.set_mat4("model", m_mat);

                if is_animated {
                    let animator = em.animators.get(*id).unwrap();
                    let animation = animator.get_current_animation().unwrap();

                    shader.set_mat4_array("bone_transforms", &animation.current_pose);

                    let cam_pos = camera.position;
                    let d2 = (trans.position - cam_pos).length_squared();
                    let do_particles = d2 < (40.0 * 40.0);

                    if do_particles {
                        for os in animation.one_shots.iter() {
                            if animation.current_segment.get() == os.segment {
                                if !os.triggered.get() {
                                    sound_manager
                                        .play_sound_3d(os.sound_type.clone(), &trans.position);
                                    particles.spawn_oneshot_emitter(
                                        "DesertStep",
                                        trans.position,
                                        None,
                                    );
                                    os.triggered.set(true);
                                }
                            } else {
                                os.triggered.set(false);
                            }
                        }
                    }

                    for cs in animation.continuous_sounds.iter() {
                        if !cs.playing.get() {
                            sound_manager.play_sound_3d(cs.sound_type.clone(), &trans.position);
                            cs.playing.set(true);
                        }
                    }

                    if let Some(fa) = &animation.hurtbox_activation {
                        if fa.segment_range.contains(&animation.current_segment.get()) {
                            if !fa.triggered.get() {
                                fa.triggered.set(true);
                            }
                        } else {
                            fa.triggered.set(false);
                        }
                    }
                }

                unsafe {
                    Self::draw_model(model, shader);
                    gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
                }
                shader.set_bool("selection_fresnel", false);
            }
        }

        unsafe {
            gl_call!(gl::Disable(gl::BLEND));
            gl::Disable(gl::CULL_FACE);
            gl_call!(gl::DepthMask(gl::TRUE));
        }

        shader.set_bool("do_reg_fresnel", false);
        shader.set_bool("selection_fresnel", false);
    }

    fn skybox_pass(&mut self, camera: &mut Camera, _fb_width: u32, _fb_height: u32) {
        unsafe {
            let skybox_shader_prog = self.shaders.get(&ShaderType::Skybox).unwrap();

            let view_no_translation = Mat4 {
                x_axis: camera.view.x_axis,
                y_axis: camera.view.y_axis,
                z_axis: camera.view.z_axis,
                w_axis: vec4(0.0, 0.0, 0.0, 1.0),
            };
            gl_call!(gl::DepthFunc(gl::LEQUAL));

            skybox_shader_prog.activate();
            skybox_shader_prog.set_mat4("view", view_no_translation);
            skybox_shader_prog.set_mat4("projection", camera.projection);

            gl_call!(gl::BindVertexArray(
                *self.vaos.get(&VaoType::Skybox).unwrap()
            ));
            gl_call!(gl::ActiveTexture(gl::TEXTURE1));
            gl_call!(gl::BindTexture(gl::TEXTURE_CUBE_MAP, self.cubemap_texture));
            gl_call!(gl::DrawElements(
                gl::TRIANGLES,
                36,
                gl::UNSIGNED_INT,
                std::ptr::null(),
            ));
            gl_call!(gl::BindVertexArray(0));

            gl_call!(gl::DepthFunc(gl::LESS));
            gl_call!(gl::BindTexture(gl::TEXTURE_CUBE_MAP, 0));
        }
    }

    fn shadow_begin(&mut self, camera: &mut Camera, light_manager: &Lights) {
        let shader = self.shaders.get_mut(&ShaderType::Depth).unwrap();

        let near_plane = light_manager.near;
        let far_plane = light_manager.far;
        let half_bound = light_manager.bounds;

        let light_dir = light_manager.dir_light.direction.normalize();
        let light_distance = light_manager.dir_light.distance;

        let camera_forward = camera.forward.normalize();
        let shadow_push = half_bound * 1.2;
        let shadow_center = camera.position + camera_forward * shadow_push;
        let light_pos = shadow_center + light_dir * light_distance;

        let light_projection = Mat4::orthographic_rh_gl(
            -half_bound,
            half_bound,
            -half_bound,
            half_bound,
            near_plane,
            far_plane,
        );
        let light_view = Mat4::look_at_rh(light_pos, shadow_center, vec3(0.0, 1.0, 0.0));

        camera.light_space = light_projection * light_view;

        shader.activate();
        shader.set_mat4("light_space_mat", camera.light_space);

        unsafe {
            gl_call!(gl::Viewport(0, 0, SHADOW_WIDTH, SHADOW_HEIGHT));
            gl_call!(gl::BindFramebuffer(
                gl::FRAMEBUFFER,
                *self.fbos.get(&FboType::DepthMap).unwrap()
            ));
            gl_call!(gl::Enable(gl::DEPTH_TEST));
            gl_call!(gl::Clear(gl::DEPTH_BUFFER_BIT));

            gl_call!(gl::Enable(CULL_FACE));
            gl_call!(gl::CullFace(gl::FRONT));
        }
    }

    fn shadow_draw_bucket(
        &mut self,
        em: &EntityManager,
        ps: &PhysicsState,
        alpha: f32,
        ids: &Vec<usize>,
        is_animated: bool,
    ) {
        self.render_sample_depth(em, ps, alpha, ids, is_animated);
    }

    fn shadow_end(&mut self) {
        unsafe {
            gl_call!(gl::CullFace(gl::BACK));
            gl_call!(gl::Disable(CULL_FACE));
            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, 0));
        }
    }

    fn render_sample_depth(
        &mut self,
        em: &EntityManager,
        _ps: &PhysicsState,
        alpha: f32,
        ids: &Vec<usize>,
        is_animated: bool,
    ) {
        let shader = self.shaders.get(&ShaderType::Depth).unwrap();
        shader.activate();
        shader.set_bool("is_animated", is_animated);

        for id in ids {
            if is_animated {
                let animator = em.animators.get(*id).unwrap();
                let animation = animator.get_current_animation().unwrap();
                shader.set_mat4_array("bone_transforms", &animation.current_pose);
            } else {
                if em.is_equipped.get(*id).is_none() && em.owners.get(*id).is_some() {
                    continue;
                }
            }

            let model = match em.models.get(*id) {
                Some(m) => m,
                None => continue,
            };
            let trans = Self::render_transform(em, *id, alpha);
            //let trans = em.transforms.get(model.key()).unwrap();

            let model_model =
                Mat4::from_scale_rotation_translation(trans.scale, trans.rotation, trans.position);
            shader.set_mat4("model", model_model);
            Self::draw_model_geometry(model);
        }
    }

    pub fn render_quad(&self) {
        unsafe {
            gl_call!(gl::BindVertexArray(
                *self.vaos.get(&VaoType::BaseQuad).unwrap()
            ));
            gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 6));
            gl_call!(gl::BindVertexArray(0));
        }
    }

    pub fn render_transform_from_args(
        _em: &EntityManager,
        curr: &Transform,
        prev: &Transform,
        alpha: f32,
    ) -> Transform {
        Transform {
            position: prev.position.lerp(curr.position, alpha),
            rotation: prev.rotation.slerp(curr.rotation, alpha),
            scale: curr.scale,
        }
    }

    pub fn render_transform(em: &EntityManager, id: usize, alpha: f32) -> Transform {
        let curr = em.transforms.get(id).unwrap();
        let prev = em.prev_transforms.get(id).unwrap_or(curr);
        Transform {
            position: prev.position.lerp(curr.position, alpha),
            rotation: prev.rotation.slerp(curr.rotation, alpha),
            scale: curr.scale,
        }
    }

    fn create_bloom_chain(&mut self, fb_width: u32, fb_height: u32) {
        let mut w = (fb_width as i32) / 2;
        let mut h = (fb_height as i32) / 2;

        self.bloom_mips.clear();

        // 5 levels of downsample
        for _level in 0..6 {
            if w < 2 || h < 2 {
                break;
            }

            let mut fbo = 0;
            let mut tex = 0;

            unsafe {
                gl_call!(gl::GenFramebuffers(1, &mut fbo));
                gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, fbo));

                gl_call!(gl::GenTextures(1, &mut tex));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, tex));

                gl_call!(gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA16F as i32,
                    w,
                    h,
                    0,
                    gl::RGBA,
                    gl::FLOAT,
                    std::ptr::null()
                ));

                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_MIN_FILTER,
                    gl::LINEAR as i32
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_MAG_FILTER,
                    gl::LINEAR as i32
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_WRAP_S,
                    gl::CLAMP_TO_EDGE as i32
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_WRAP_T,
                    gl::CLAMP_TO_EDGE as i32
                ));

                gl_call!(gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0,
                    gl::TEXTURE_2D,
                    tex,
                    0
                ));

                let bufs = [gl::COLOR_ATTACHMENT0];
                gl_call!(gl::DrawBuffers(1, bufs.as_ptr()));

                // optionally assert framebuffer complete
                gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, 0));
            }

            self.bloom_mips.push(BloomMip { fbo, tex, w, h });

            w /= 2;
            h /= 2;
        }
    }

    fn bloom_down_up(&mut self, fb_width: u32, fb_height: u32) -> u32 {
        unsafe {
            gl_call!(gl::Disable(gl::DEPTH_TEST));
        }

        // ---- downsample ----
        {
            let s = self.shaders.get_mut(&ShaderType::BloomDownsample).unwrap();
            s.activate();
            s.set_int("src", 0);
        }

        let mut src_tex = self.hdr_bright;
        let mut src_w = fb_width as f32;
        let mut src_h = fb_height as f32;

        for mip in &self.bloom_mips {
            {
                let s = self.shaders.get_mut(&ShaderType::BloomDownsample).unwrap();
                s.set_vec2("texelSize", 1.0 / src_w, 1.0 / src_h);
            }

            unsafe {
                gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, mip.fbo));
                let bufs = [gl::COLOR_ATTACHMENT0];
                gl_call!(gl::DrawBuffers(1, bufs.as_ptr()));
                gl_call!(gl::Viewport(0, 0, mip.w, mip.h));
                gl_call!(gl::ActiveTexture(gl::TEXTURE0));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, src_tex));
            }

            self.render_quad();

            src_tex = mip.tex;
            src_w = mip.w as f32;
            src_h = mip.h as f32;
        }

        // ---- upsample ----
        unsafe {
            gl_call!(gl::Enable(gl::BLEND));
            gl_call!(gl::BlendFunc(gl::ONE, gl::ONE));
        }

        {
            let s = self.shaders.get_mut(&ShaderType::BloomUpsample).unwrap();
            s.activate();
            s.set_int("src", 0);
        }

        for i in (1..self.bloom_mips.len()).rev() {
            let small = &self.bloom_mips[i];
            let big = &self.bloom_mips[i - 1];

            {
                let s = self.shaders.get_mut(&ShaderType::BloomUpsample).unwrap();
                s.set_vec2("texelSize", 1.0 / small.w as f32, 1.0 / small.h as f32);
            }

            unsafe {
                gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, big.fbo));
                gl_call!(gl::Viewport(0, 0, big.w, big.h));
                let bufs = [gl::COLOR_ATTACHMENT0];
                gl_call!(gl::DrawBuffers(1, bufs.as_ptr()));
                gl_call!(gl::ActiveTexture(gl::TEXTURE0));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, small.tex));
            }

            self.render_quad();
        }

        unsafe {
            gl_call!(gl::Disable(gl::BLEND));
            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, 0));
        }

        self.bloom_mips[0].tex
    }
}
