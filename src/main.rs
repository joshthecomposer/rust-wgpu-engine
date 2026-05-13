mod abilities;
mod animation;
mod assets;
mod camera;
mod config;
mod damage_resolution_system;
mod debug;
mod entity_manager;
mod enums_types;
mod game;
mod input;
mod items;
mod lights;
mod macros;
mod movement_system;
mod particles;
mod physics;
mod platform;
mod sound;
mod sparse_set;
mod spawn_system;
mod state_machines;
mod terrain;
mod time;
mod ui;
mod util;

mod command_buffer;
mod world;

mod damage_volume_spawn_system;
mod projectile_system;
mod status_effect_system;
mod wgpu_backend;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use config::{game_config::GameConfig, Config};
use game::Game;

use crate::platform::Platform;

use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;
use winit::{application::ApplicationHandler, event::DeviceEvent};

use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use winit::window::Window;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Monotonic clock for both native (`std::time::Instant`) and wasm (`performance.now()`).
struct Clock {
    #[cfg(not(target_arch = "wasm32"))]
    start: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    start_ms: f64,
}

impl Clock {
    fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            start: std::time::Instant::now(),
            #[cfg(target_arch = "wasm32")]
            start_ms: web_sys::window()
                .and_then(|w| w.performance())
                .map(|p| p.now())
                .unwrap_or(0.0),
        }
    }

    fn elapsed_secs(&self) -> f32 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.start.elapsed().as_secs_f32()
        }
        #[cfg(target_arch = "wasm32")]
        {
            let now = web_sys::window()
                .and_then(|w| w.performance())
                .map(|p| p.now())
                .unwrap_or(self.start_ms);
            ((now - self.start_ms) * 0.001) as f32
        }
    }
}

struct App {
    // Shared so the wasm async init task can store the `Game` once the WebGPU
    // adapter/device futures resolve. Native fills it synchronously.
    game: Rc<RefCell<Option<Game>>>,
    window: Option<Arc<Window>>,
    start: Clock,
    config: Option<GameConfig>,
}

impl App {
    fn new(config: GameConfig) -> Self {
        Self {
            game: Rc::new(RefCell::new(None)),
            window: None,
            start: Clock::new(),
            config: Some(config),
        }
    }
}

impl ApplicationHandler for App {
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit(); // TODO: Put this in the message queue like it was in glfw
            }
            WindowEvent::RedrawRequested => {
                if self.window.as_ref().is_some_and(|w| w.id() == window_id) {
                    if let Some(game) = self.game.borrow_mut().as_mut() {
                        let now = self.start.elapsed_secs();
                        game.tick(now);
                    }
                }
            }
            WindowEvent::Resized(new_size) => {
                if let Some(game) = self.game.borrow_mut().as_mut() {
                    game.resize(new_size.width, new_size.height);
                }
            }
            _ => {
                if let Some(game) = self.game.borrow_mut().as_mut() {
                    game.handle_window_event(&event);
                }
            }
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let config = self
            .config
            .take()
            .expect("App::resumed called after game was already initialized");

        let mut window_attrs = Window::default_attributes()
            .with_title("best dang game")
            .with_inner_size(LogicalSize::new(
                config.win_width as u32,
                config.win_height as u32,
            ))
            .with_resizable(true);

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            let canvas = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("game-canvas"))
                .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok());
            window_attrs = window_attrs.with_canvas(canvas);
        }

        let window = Arc::new(event_loop.create_window(window_attrs).unwrap());

        self.window = Some(Arc::clone(&window));

        let inner = window.inner_size();
        let platform = Platform {
            fb_width: inner.width.max(1),
            fb_height: inner.height.max(1),
            window: Some(Arc::clone(&window)),
        };

        let game_slot = Rc::clone(&self.game);

        // On native we can block on the WebGPU/Vulkan device init. On the web the
        // WebGPU adapter/device requests are JS Promises that only resolve once we
        // yield back to the browser event loop, so `block_on` would deadlock the
        // single wasm thread — drive them on the microtask queue instead.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let game = pollster::block_on(Game::new(platform, config));
            *game_slot.borrow_mut() = Some(game);
            window.request_redraw();
        }

        #[cfg(target_arch = "wasm32")]
        {
            let window_for_redraw = Arc::clone(&window);
            wasm_bindgen_futures::spawn_local(async move {
                let game = Game::new(platform, config).await;
                *game_slot.borrow_mut() = Some(game);
                window_for_redraw.request_redraw();
            });
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            // delta: (dx, dy) in f64
            let (dx, dy) = delta;
            // only process camera input when not paused AND cursor is locked
            if let Some(game) = self.game.borrow_mut().as_mut() {
                if !game.paused {
                    game.world.camera.process_mouse_input(dx, dy);
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let config = GameConfig::load_or_create_default("config/game_config.json");
    let mut app = App::new(config);

    let event_loop = EventLoop::new().unwrap();

    event_loop.run_app(&mut app).expect("event loop error");
}

#[cfg(target_arch = "wasm32")]
fn main() {}

/// Browser entry point. The wasm-bindgen `start` attribute makes this fire as
/// soon as `init()` from learn_opengl_rs.js finishes instantiating the module,
/// matching the `await init()` call in web/index.html.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn wasm_main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let config = GameConfig::load_or_create_default("config/game_config.json");
    let app = App::new(config);

    let event_loop = EventLoop::new().map_err(|e| JsValue::from_str(&e.to_string()))?;

    use winit::platform::web::EventLoopExtWebSys;
    event_loop.spawn_app(app);
    Ok(())
}
