use glfw::Context;

pub struct Platform {
    pub glfw: glfw::Glfw,
    pub window: glfw::PWindow,
    pub events: glfw::GlfwReceiver<(f64, glfw::WindowEvent)>,
    pub fb_width: u32,
    pub fb_height: u32,
}

impl Platform {
    pub fn new(title: &str, w: u32, h: u32, vsync: bool) -> Self {
        let mut glfw = glfw::init(glfw::fail_on_errors).expect("Failed to init glfw");

        glfw.window_hint(glfw::WindowHint::ContextVersion(4, 1)); // OpenGL 3.3
        glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));
        glfw.window_hint(glfw::WindowHint::Resizable(true));

        #[cfg(target_os = "macos")]
        glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));

        glfw.window_hint(glfw::WindowHint::Samples(Some(16)));

        let (mut window, events) = glfw
            .create_window(w, h, title, glfw::WindowMode::Windowed)
            .expect("Failed to create GLFW window.");

        window.set_cursor_mode(glfw::CursorMode::Disabled);
        window.set_all_polling(true);
        window.set_framebuffer_size_polling(true);
        window.make_current();

        glfw.set_swap_interval(if vsync {
            glfw::SwapInterval::Sync(1)
        } else {
            glfw::SwapInterval::None
        });

        let (fb_width, fb_height) = window.get_framebuffer_size();

        gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

        Self {
            glfw,
            window,
            events,
            fb_width: fb_width as u32,
            fb_height: fb_height as u32,
        }
    }
}
