#[cfg(all(
    feature = "native_audio",
    any(target_os = "macos", target_os = "windows"),
    not(target_arch = "wasm32")
))]
pub mod fmod;

#[cfg(all(target_arch = "wasm32", feature = "web_audio"))]
pub mod web_fmod_bridge;

pub mod sound_manager;
