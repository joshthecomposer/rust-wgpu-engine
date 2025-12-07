//! UI Events - Structured events from the UI system
//!
//! These events are parsed from raw JS/JSON events in the UiManager
//! and passed to Game for handling.

/// Structured UI event types that Game can handle.
#[derive(Debug, Clone)]
pub enum UiEvent {
    /// Light direction update (x, y, z, distance)
    LightUpdate {
        x: Option<f32>,
        y: Option<f32>,
        z: Option<f32>,
        distance: Option<f32>,
    },

    /// Shadow debug toggle
    ShadowDebug { enabled: bool },

    /// Orthographic projection update
    OrthoUpdate {
        near: Option<f32>,
        far: Option<f32>,
        bounds: Option<f32>,
        bias: Option<f32>,
    },

    /// Master volume update
    VolumeUpdate { volume: f32 },

    /// Sound toggle (pause/resume)
    SoundToggle { paused: bool },

    /// Sound pause
    SoundPause,

    /// Sound play
    SoundPlay,

    /// Create mode toggle with entity creation parameters
    CreateModeToggle {
        enabled: bool,
        entity_type: String,
        faction: String,
        weapon: Option<String>,
        base_speed: f32,
    },

    /// Emitter preview rendering toggle (raw JSON preserved for complex parsing)
    RenderEmitterPreview {
        enabled: bool,
        /// Raw JSON for emitter parsing (Game handles the complex parsing)
        raw_json: String,
    },

    /// Save emitter to file (raw JSON preserved for complex parsing)
    SaveEmitter {
        /// Raw JSON for emitter parsing (Game handles the complex parsing)
        raw_json: String,
    },

    /// Pause menu: close/resume
    PauseResume,

    /// Pause menu: toggle gizmos
    PauseToggleGizmos,

    /// Pause menu: reload world data
    PauseReloadWorld,

    /// Pause menu: save player data
    PauseSavePlayer,

    /// Pause menu: quit game
    PauseQuit,

    /// Unknown event type
    Unknown { event_type: String },
}

/// Parse a raw JSON event string into a structured UiEvent.
pub fn parse_js_event(event_json: &str) -> Option<UiEvent> {
    let event: serde_json::Value = match serde_json::from_str(event_json) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[UiEvent] Failed to parse JS event: {} - {}", e, event_json);
            return None;
        }
    };

    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");
    println!("[UiEvent] Parsing event: {}", event_type);

    let ui_event = match event_type {
        "light_update" => parse_light_update(&event),
        "shadow_debug" => parse_shadow_debug(&event),
        "ortho_update" => parse_ortho_update(&event),
        "volume_update" => parse_volume_update(&event),
        "sound_toggle" => parse_sound_toggle(&event),
        "sound_pause" => Some(UiEvent::SoundPause),
        "sound_play" => Some(UiEvent::SoundPlay),
        "create_mode_toggle" => parse_create_mode_toggle(&event),
        "render_emitter_preview" => parse_render_emitter_preview(&event),
        "save_emitter" => parse_save_emitter(&event),
        "pause_close" | "pause_resume" => Some(UiEvent::PauseResume),
        "pause_toggle_gizmos" => Some(UiEvent::PauseToggleGizmos),
        "pause_reload_world" => Some(UiEvent::PauseReloadWorld),
        "pause_save_player" => Some(UiEvent::PauseSavePlayer),
        "pause_quit" => Some(UiEvent::PauseQuit),
        _ => Some(UiEvent::Unknown { event_type: event_type.to_string() }),
    };

    ui_event
}

fn parse_light_update(event: &serde_json::Value) -> Option<UiEvent> {
    let data = event.get("data")?;
    Some(UiEvent::LightUpdate {
        x: data.get("x").and_then(|v| v.as_f64()).map(|v| v as f32),
        y: data.get("y").and_then(|v| v.as_f64()).map(|v| v as f32),
        z: data.get("z").and_then(|v| v.as_f64()).map(|v| v as f32),
        distance: data.get("distance").and_then(|v| v.as_f64()).map(|v| v as f32),
    })
}

fn parse_shadow_debug(event: &serde_json::Value) -> Option<UiEvent> {
    let enabled = event.get("data")?.get("enabled")?.as_bool()?;
    Some(UiEvent::ShadowDebug { enabled })
}

fn parse_ortho_update(event: &serde_json::Value) -> Option<UiEvent> {
    let data = event.get("data")?;
    Some(UiEvent::OrthoUpdate {
        near: data.get("near").and_then(|v| v.as_f64()).map(|v| v as f32),
        far: data.get("far").and_then(|v| v.as_f64()).map(|v| v as f32),
        bounds: data.get("bounds").and_then(|v| v.as_f64()).map(|v| v as f32),
        bias: data.get("bias").and_then(|v| v.as_f64()).map(|v| v as f32),
    })
}

fn parse_volume_update(event: &serde_json::Value) -> Option<UiEvent> {
    let volume = event.get("data")?.get("volume")?.as_f64()? as f32;
    Some(UiEvent::VolumeUpdate { volume })
}

fn parse_sound_toggle(event: &serde_json::Value) -> Option<UiEvent> {
    let paused = event.get("data")?.get("paused")?.as_bool()?;
    Some(UiEvent::SoundToggle { paused })
}

fn parse_create_mode_toggle(event: &serde_json::Value) -> Option<UiEvent> {
    let enabled = event.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    if enabled {
        Some(UiEvent::CreateModeToggle {
            enabled: true,
            entity_type: event.get("entityType").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            faction: event.get("faction").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            weapon: event.get("weapon").and_then(|v| v.as_str()).map(|s| s.to_string()),
            base_speed: event.get("baseSpeed").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
        })
    } else {
        Some(UiEvent::CreateModeToggle {
            enabled: false,
            entity_type: String::new(),
            faction: String::new(),
            weapon: None,
            base_speed: 0.0,
        })
    }
}

fn parse_render_emitter_preview(event: &serde_json::Value) -> Option<UiEvent> {
    let enabled = event.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    // Preserve the raw JSON for Game to parse (complex EmitterBlackboard parsing)
    let raw_json = serde_json::to_string(event).unwrap_or_default();
    Some(UiEvent::RenderEmitterPreview { enabled, raw_json })
}

fn parse_save_emitter(event: &serde_json::Value) -> Option<UiEvent> {
    let name = event.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if name.is_empty() {
        eprintln!("[UiEvent] Cannot save emitter: name is empty");
        return None;
    }
    // Preserve the raw JSON for Game to parse (complex EmitterBlackboard parsing)
    let raw_json = serde_json::to_string(event).unwrap_or_default();
    Some(UiEvent::SaveEmitter { raw_json })
}
