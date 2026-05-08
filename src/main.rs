mod abilities;
mod assets;
mod camera;
mod shaders;
// mod game_state;
mod animation;
mod config;
mod debug;
mod entity_manager;
mod enums_types;
mod grid;
mod input;
mod lights;
mod macros;
mod movement_system;
mod renderer;
mod sound;
mod sparse_set;
mod terrain;
mod ui;
// mod deprecated;
mod damage_resolution_system;
mod game;
mod items;
mod particles;
mod physics;
mod platform;
mod spawn_system;
mod state_machines;
mod time;
mod util;
#[cfg(target_arch = "wasm32")]
mod web_game;

mod command_buffer;
mod world;

mod damage_volume_spawn_system;
mod projectile_system;

#[cfg(not(target_arch = "wasm32"))]
use config::{game_config::GameConfig, Config};
#[cfg(not(target_arch = "wasm32"))]
use game::Game;

#[cfg(not(target_arch = "wasm32"))]
use crate::platform::Platform;

#[cfg(not(target_arch = "wasm32"))]
use winit::event::WindowEvent;
#[cfg(not(target_arch = "wasm32"))]
use winit::event_loop::ActiveEventLoop;
#[cfg(not(target_arch = "wasm32"))]
use winit::window::WindowId;
#[cfg(not(target_arch = "wasm32"))]
use winit::{application::ApplicationHandler, event::DeviceEvent};

#[cfg(not(target_arch = "wasm32"))]
struct App {
    game: Game,
    window_id: WindowId,
    start: std::time::Instant,
}

#[cfg(not(target_arch = "wasm32"))]
impl App {
    fn new(game: Game) -> Self {
        let window_id = game.platform.window.id();
        Self {
            game,
            window_id,
            start: std::time::Instant::now(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ApplicationHandler for App {
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if window_id != self.window_id {
            return;
        }

        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit(); // TODO: Put this in the message queue like it was in glfw
            }
            _ => {
                self.game.handle_window_event(&event);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // This runs every time winit is about to sleep
        let now = self.start.elapsed().as_secs_f32();
        self.game.tick(now);

        if self.game.should_quit {
            event_loop.exit();
            return;
        }

        // Continuous redraw
        self.game.platform.window.request_redraw();
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

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
            if !self.game.paused && !self.game.cursor_unlocked() {
                self.game.world.camera.process_mouse_input(dx, dy);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let config = GameConfig::load_or_create_default("config/game_config.json");

    let (platform, event_loop) = Platform::new(
        &config.game_title,
        config.win_width as u32,
        config.win_height as u32,
        config.vsync,
    );
    platform.window.focus_window();
    let game = Game::new(platform, config);
    let mut app = App::new(game);

    event_loop.run_app(&mut app).expect("event loop error");
}

#[cfg(target_arch = "wasm32")]
fn main() {}
