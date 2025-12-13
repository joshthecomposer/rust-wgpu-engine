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
mod some_data;
mod sound;
mod sparse_set;
mod terrain;
mod ui;
mod uniforms;
// mod deprecated;
mod combat_system;
mod game;
mod items;
mod particles;
mod physics;
mod platform;
mod state_machines;
mod time;
mod util;

mod world;
use std::{
    fs::{self, OpenOptions},
    path::Path,
};

use game::Game;

use glam::Quat;
use std::io::Write;

use crate::platform::Platform;

use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;
use winit::{application::ApplicationHandler, event::DeviceEvent};

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

        // Check if game wants to quit
        if self.game.should_quit() {
            event_loop.exit();
            return;
        }

        // Continuous redraw
        self.game.platform.window.request_redraw();
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {}

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
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

    let game = Game::new(platform);
    let mut app = App::new(game);

    event_loop.run_app(&mut app).expect("event loop error");
}
