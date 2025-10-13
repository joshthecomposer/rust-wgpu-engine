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
//mod items;
mod physics;
mod util;
mod combat_system;
mod game;
mod time;
mod platform;

mod world;
use std::{fs::{self, OpenOptions}, path::Path};

use game::Game;

use glam::Quat;
use std::io::Write;

fn main() {
    let mut game = Game::new();
    game.run();
}
