use crate::{config::Config, enums_types::FxaaLevels};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct GameConfig {
    pub game_title: String,
    pub cell_size: f32,
    pub win_width: f32,
    pub win_height: f32,
    pub window_mode: String,
    pub grid_height: usize,
    pub grid_width: usize,
    pub vsync: bool,
    pub debug_mode: bool,
    pub fps_counter: bool,
    pub render_gizmos: bool,
    #[serde(default)]
    pub shadow_debug: bool,
    pub msaa_level: i32,
    pub fxaa_level: FxaaLevels,
    pub font_family: String,
    pub spawn_system_enabled: bool,
    #[serde(default)]
    pub webgl_compatibility_mode: bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            game_title: "Spaghetti Engine".to_string(),
            cell_size: 1.0,
            win_width: 1280.0,
            win_height: 720.0,
            window_mode: "Windowed".to_string(),
            grid_height: 100,
            grid_width: 100,
            vsync: true,
            debug_mode: true,
            fps_counter: true,
            render_gizmos: false,
            shadow_debug: false,
            msaa_level: 16,
            fxaa_level: FxaaLevels::Off,
            font_family: "Weiholmir".to_string(),
            spawn_system_enabled: true,
            webgl_compatibility_mode: false,
        }
    }
}

impl Config for GameConfig {}
