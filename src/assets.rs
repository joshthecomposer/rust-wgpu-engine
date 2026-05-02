#![allow(dead_code)]

#[cfg(not(target_arch = "wasm32"))]
use std::{fs, path::Path};

use image::DynamicImage;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

pub fn read_text(path: &str) -> std::io::Result<String> {
    let bytes = read_bytes(path)?;
    String::from_utf8(bytes).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("asset is not valid UTF-8: {path}: {error}"),
        )
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_bytes(path: &str) -> std::io::Result<Vec<u8>> {
    fs::read(path)
}

#[cfg(target_arch = "wasm32")]
pub fn read_bytes(path: &str) -> std::io::Result<Vec<u8>> {
    embedded_asset(path)
        .map(|bytes| bytes.to_vec())
        .or_else(|| preloaded_browser_asset(path))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("embedded browser asset is not available: {path}"),
            )
        })
}

pub fn load_image(path: &str) -> Result<DynamicImage, String> {
    let bytes =
        read_bytes(path).map_err(|error| format!("Failed to read image {path}: {error}"))?;
    image::load_from_memory(&bytes)
        .map_err(|error| format!("Failed to decode image {path}: {error}"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn path_exists(path: &str) -> bool {
    Path::new(path).exists()
}

#[cfg(target_arch = "wasm32")]
pub fn path_exists(path: &str) -> bool {
    embedded_asset(path).is_some() || preloaded_browser_asset(path).is_some()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_text(path: &str, contents: &str) -> std::io::Result<()> {
    fs::write(path, contents)
}

#[cfg(target_arch = "wasm32")]
pub fn write_text(path: &str, _contents: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        format!("browser config writes are not implemented yet: {path}"),
    ))
}

#[cfg(target_arch = "wasm32")]
fn embedded_asset(path: &str) -> Option<&'static [u8]> {
    match path {
        "config/game_config.json" => Some(DEFAULT_GAME_CONFIG_JSON.as_bytes()),
        "resources/shaders/custom_ui_es300.glsl" => {
            Some(include_bytes!("../resources/shaders/custom_ui_es300.glsl"))
        }
        "resources/shaders/depth_shader_es300.glsl" => Some(include_bytes!(
            "../resources/shaders/depth_shader_es300.glsl"
        )),
        "resources/shaders/skybox_es300.glsl" => {
            Some(include_bytes!("../resources/shaders/skybox_es300.glsl"))
        }
        "resources/shaders/model/static_model_es300.glsl" => Some(include_bytes!(
            "../resources/shaders/model/static_model_es300.glsl"
        )),
        "resources/shaders/model/animated_model_es300.glsl" => Some(include_bytes!(
            "../resources/shaders/model/animated_model_es300.glsl"
        )),
        "resources/shaders/minimal_world_es300.glsl" => Some(include_bytes!(
            "../resources/shaders/minimal_world_es300.glsl"
        )),
        "resources/shaders/web_smoke_scene_es300.glsl" => Some(include_bytes!(
            "../resources/shaders/web_smoke_scene_es300.glsl"
        )),
        "resources/shaders/web_game_scene_es300.glsl" => Some(include_bytes!(
            "../resources/shaders/web_game_scene_es300.glsl"
        )),
        _ => None,
    }
}

#[cfg(target_arch = "wasm32")]
fn preloaded_browser_asset(path: &str) -> Option<Vec<u8>> {
    let window = web_sys::window()?;
    let asset_map = js_sys::Reflect::get(
        window.as_ref(),
        &JsValue::from_str("__learn_opengl_rs_assets"),
    )
    .ok()?;
    let bytes = js_sys::Reflect::get(&asset_map, &JsValue::from_str(path)).ok()?;

    if bytes.is_undefined() || bytes.is_null() {
        return None;
    }

    Some(js_sys::Uint8Array::new(&bytes).to_vec())
}

#[cfg(target_arch = "wasm32")]
const DEFAULT_GAME_CONFIG_JSON: &str = r#"{
  "game_title": "Spaghetti Engine",
  "cell_size": 1.0,
  "win_width": 1280.0,
  "win_height": 720.0,
  "window_mode": "Windowed",
  "grid_height": 100,
  "grid_width": 100,
  "vsync": true,
  "debug_mode": true,
  "fps_counter": true,
  "render_gizmos": false,
  "msaa_level": 16,
  "fxaa_level": "Off",
  "font_family": "Weiholmir",
  "spawn_system_enabled": true,
  "webgl_compatibility_mode": true
}"#;
