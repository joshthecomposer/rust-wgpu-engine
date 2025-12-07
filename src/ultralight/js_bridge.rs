//! JavaScript ↔ Rust bridge for bidirectional communication.
//!
//! This module handles:
//! - Exposing Rust functions callable from JavaScript
//! - Queuing events from JavaScript to be processed in the game loop
//! - Syncing game state to JavaScript for UI updates

#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use super::types::{JsEvent, GameEvent, ExposedGameState, UltralightError};

/// Type alias for Rust functions callable from JavaScript.
pub type JsCallable = Box<dyn Fn(&[String]) -> Result<String, String> + Send>;

/// Bridge for JS ↔ Rust communication.
pub struct JsBridge {
    /// Queue of events from JS to be processed by Rust
    event_queue: VecDeque<JsEvent>,
    /// Registered Rust functions callable from JS
    registered_functions: HashMap<String, JsCallable>,
    /// Current game state exposed to JS
    exposed_state: ExposedGameState,
    /// Pending events to send to JS
    pending_game_events: VecDeque<GameEvent>,
}

impl Default for JsBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl JsBridge {
    pub fn new() -> Self {
        Self {
            event_queue: VecDeque::new(),
            registered_functions: HashMap::new(),
            exposed_state: ExposedGameState::default(),
            pending_game_events: VecDeque::new(),
        }
    }

    /// Register a Rust function that can be called from JavaScript.
    ///
    /// The function receives an array of string arguments and returns a string result.
    ///
    /// # Example
    /// ```rust,ignore
    /// bridge.register_function("spawnEntity", |args| {
    ///     let entity_type = args.get(0).ok_or("Missing entity type")?;
    ///     // ... spawn entity logic
    ///     Ok("entity_id_123".to_string())
    /// });
    /// ```
    pub fn register_function<F>(&mut self, name: &str, func: F)
    where
        F: Fn(&[String]) -> Result<String, String> + Send + 'static,
    {
        self.registered_functions.insert(name.to_string(), Box::new(func));
    }

    /// Call a registered function (invoked by JS bridge code).
    pub fn call_function(&self, name: &str, args: &[String]) -> Result<String, String> {
        if let Some(func) = self.registered_functions.get(name) {
            func(args)
        } else {
            Err(format!("Function '{}' not registered", name))
        }
    }

    /// Queue an event from JavaScript to be processed by Rust.
    pub fn queue_event(&mut self, event: JsEvent) {
        self.event_queue.push_back(event);
    }

    /// Drain all queued JS events for processing in the game loop.
    pub fn drain_events(&mut self) -> Vec<JsEvent> {
        self.event_queue.drain(..).collect()
    }

    /// Check if there are pending events.
    pub fn has_pending_events(&self) -> bool {
        !self.event_queue.is_empty()
    }

    /// Update the exposed game state that JS can read.
    pub fn sync_from_game(&mut self, state: ExposedGameState) {
        self.exposed_state = state;
    }

    /// Get the current exposed state (for JS to read).
    pub fn get_exposed_state(&self) -> &ExposedGameState {
        &self.exposed_state
    }

    /// Queue a game event to be sent to JavaScript.
    pub fn send_event(&mut self, event: GameEvent) {
        self.pending_game_events.push_back(event);
    }

    /// Drain pending game events to send to JS views.
    pub fn drain_game_events(&mut self) -> Vec<GameEvent> {
        self.pending_game_events.drain(..).collect()
    }

    /// Generate the JavaScript code to inject into views.
    ///
    /// This creates a `RustBridge` object in JavaScript with methods to:
    /// - Call registered Rust functions
    /// - Emit events to Rust
    /// - Get current game state
    pub fn generate_js_bindings(&self) -> String {
        let function_names: Vec<&String> = self.registered_functions.keys().collect();
        
        format!(r#"
window.RustBridge = {{
    // Registered function names: {:?}
    
    call: function(funcName, ...args) {{
        // This will be replaced by actual Ultralight JS bindings
        console.log('[RustBridge] Calling:', funcName, args);
        return window.__ultralight_call_rust(funcName, JSON.stringify(args));
    }},
    
    emit: function(eventType, data) {{
        console.log('[RustBridge] Emitting:', eventType, data);
        window.__ultralight_emit_event(eventType, JSON.stringify(data));
    }},
    
    getState: function() {{
        return JSON.parse(window.__ultralight_get_state());
    }},
    
    onGameEvent: function(callback) {{
        window.__ultralight_game_event_callback = callback;
    }}
}};

console.log('[RustBridge] Initialized');
"#, function_names)
    }
}

