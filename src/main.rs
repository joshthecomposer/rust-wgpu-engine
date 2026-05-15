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
//mod particles;
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

struct App {
    game: Option<Game>,
    window: Option<Arc<Window>>,
    start: std::time::Instant,
    config: Option<GameConfig>,
}

impl App {
    fn new(config: GameConfig) -> Self {
        Self {
            game: None,
            window: None,
            start: std::time::Instant::now(),
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
                    if let Some(game) = &mut self.game {
                        let now = self.start.elapsed().as_secs_f32();
                        game.tick(now);
                    }
                }
            }
            _ => {
                if let Some(game) = &mut self.game {
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

        let window_attrs = Window::default_attributes()
            .with_title("best dang game")
            .with_inner_size(LogicalSize::new(
                config.win_width as u32,
                config.win_height as u32,
            ))
            .with_resizable(true);

        let window = Arc::new(event_loop.create_window(window_attrs).unwrap());

        self.window = Some(Arc::clone(&window));

        let platform = Platform {
            fb_width: config.win_width as u32,
            fb_height: config.win_height as u32,
            window: Some(Arc::clone(&window)),
        };
        let game = Game::new(platform, config);
        self.game = Some(game);
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
            if let Some(game) = &mut self.game {
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
