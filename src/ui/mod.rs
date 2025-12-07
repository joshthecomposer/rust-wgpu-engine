pub mod font;
pub mod color;
pub mod message_queue;
pub mod ui_manager;
pub mod events;

// DEPRECATED: ImGui module has been replaced by Ultralight HTML/CSS/JS UI system.
// The code is preserved in deprecated_imgui/ for reference but is no longer compiled.
// pub mod deprecated_imgui;

// DEPRECATED: game_ui module has been replaced by Ultralight HTML/CSS/JS UI system.
// The code is preserved in deprecated_game_ui/ for reference.
// Only GameUiContext is still used for texture caching.
pub mod deprecated_game_ui;
