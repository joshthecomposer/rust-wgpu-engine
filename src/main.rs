mod shaders;
mod camera;
// mod game_state;
mod some_data;
mod macros;
mod enums_types;
mod sparse_set;
mod uniforms;
mod entity_manager;
mod lights;
mod grid;
mod renderer;
mod animation;
mod debug;
mod input;
mod movement_system;
mod ui;
mod sound;
mod config;
mod terrain;
// mod deprecated;
mod state_machines;
mod particles;
mod items;
mod physics;
mod util;
mod combat_system;
mod game;
mod time;
mod platform;
mod grounding_solver;
mod ultralight;

mod world;

use game::Game;

use crate::platform::Platform;
use crate::ui::ui_manager::UiManager;

use winit::{application::ApplicationHandler, event::DeviceEvent};
use winit::event::{WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

struct App {
    game: Game,
    window_id: WindowId,
    start: std::time::Instant,
}

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

        // Check if game requested to close (e.g., from pause menu quit button)
        if self.game.should_close {
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
            if !self.game.paused {
                self.game.world.camera.process_mouse_input(dx, dy);
            }
        }
    }
}

fn main() {
    let (platform, event_loop) = Platform::new("Spaghetti engine", 1280, 720, false);

    // Initialize UI manager with Ultralight views
    let ui_manager = UiManager::new(platform.fb_width, platform.fb_height);

    let game = Game::new(platform, ui_manager);
    let mut app = App::new(game);

    event_loop.run_app(&mut app).expect("event loop error");
}
