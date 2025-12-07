#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use winit::event::WindowEvent;

use ul_next::{
    config::Config,
    event::{
        KeyEvent, KeyEventCreationInfo, KeyEventModifiers, KeyEventType,
        MouseButton as UlMouseButton, MouseEvent, MouseEventType, ScrollEvent, ScrollEventType,
    },
    key_code::VirtualKeyCode,
    platform::{self, LogLevel, Logger},
    renderer::Renderer,
    view::ViewConfig as UlViewConfig,
    Library,
};

use crate::shaders::Shader;

use super::input_adapter::{InputAdapter, UltralightInputEvent, UltralightMouseButton};
use super::js_bridge::JsBridge;
use super::types::{ExposedGameState, GameEvent, UltralightError, ViewType};
use super::view::UltralightView;

/// Custom logger for Ultralight messages.
struct UltralightLogger;

impl Logger for UltralightLogger {
    fn log_message(&mut self, log_level: LogLevel, message: String) {
        match log_level {
            LogLevel::Error => eprintln!("[Ultralight ERROR] {}", message),
            LogLevel::Warning => eprintln!("[Ultralight WARN] {}", message),
            LogLevel::Info => println!("[Ultralight INFO] {}", message),
        }
    }
}

/// Main manager for Ultralight UI rendering.
///
/// Handles creation and management of views, input routing, and rendering.
pub struct UltralightManager {
    /// Ultralight library handle
    lib: Arc<Library>,
    /// Ultralight renderer
    renderer: Renderer,
    /// All created views, indexed by ViewType
    views: HashMap<ViewType, UltralightView>,
    /// Ordered list of visible views for rendering (back to front)
    render_order: Vec<ViewType>,
    /// Currently focused view (receives keyboard input)
    focused_view: Option<ViewType>,
    /// Input adapter for converting winit events
    input_adapter: InputAdapter,
    /// JavaScript bridge for Rust ↔ JS communication
    js_bridge: JsBridge,
    /// Whether any view wants to capture mouse input
    pub want_capture_mouse: bool,
    /// Whether any view wants to capture keyboard input
    pub want_capture_keyboard: bool,
    /// Whether the system is initialized
    initialized: bool,
    /// OpenGL texture for rendering CPU views
    gl_texture: u32,
    /// Texture dimensions
    texture_size: (u32, u32),
    /// Shader for rendering UI textures
    shader: Shader,
    /// VAO for quad rendering
    vao: u32,
    /// VBO for quad rendering
    vbo: u32,
}

impl UltralightManager {
    /// Create a new UltralightManager.
    ///
    /// This initializes the Ultralight library, sets up platform handlers,
    /// and creates the renderer for CPU-based rendering.
    pub fn new(width: u32, height: u32) -> Result<Self, UltralightError> {
        println!("[Ultralight] Initializing UltralightManager...");

        // Get the Ultralight library handle
        let lib = Library::linked();

        // Set up platform handlers
        // Font loader is required
        platform::enable_platform_fontloader(lib.clone());

        // Set up filesystem with root at current directory
        platform::enable_platform_filesystem(lib.clone(), ".")
            .map_err(|e| UltralightError::InitializationFailed(format!("Failed to enable filesystem: {:?}", e)))?;

        // Set up logger
        platform::set_logger(lib.clone(), UltralightLogger);

        // Create config for CPU rendering (non-accelerated)
        let config = Config::start()
            .build(lib.clone())
            .ok_or_else(|| UltralightError::InitializationFailed("Failed to create config".to_string()))?;

        // Create the renderer
        let renderer = Renderer::create(config)
            .map_err(|e| UltralightError::InitializationFailed(format!("Failed to create renderer: {:?}", e)))?;

        // Create OpenGL texture for rendering
        let gl_texture = unsafe {
            let mut texture: u32 = 0;
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            // Allocate texture storage (BGRA format from Ultralight)
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as i32,
                width as i32,
                height as i32,
                0,
                gl::BGRA,
                gl::UNSIGNED_BYTE,
                std::ptr::null(),
            );
            gl::BindTexture(gl::TEXTURE_2D, 0);
            texture
        };

        // Load the UI shader
        let shader = Shader::new("resources/shaders/ultralight_ui.glsl");
        shader.activate();
        shader.set_int("uTexture", 0);

        // Create VAO/VBO for fullscreen quad
        let (vao, vbo) = unsafe {
            let mut vao: u32 = 0;
            let mut vbo: u32 = 0;
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);

            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            // Allocate space for dynamic quad vertices (6 vertices * 5 floats)
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (6 * 5 * std::mem::size_of::<f32>()) as isize,
                std::ptr::null(),
                gl::DYNAMIC_DRAW,
            );

            let stride = (5 * std::mem::size_of::<f32>()) as i32;

            // Position attribute (location 0)
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, std::ptr::null());

            // Texture coordinate attribute (location 1)
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (3 * std::mem::size_of::<f32>()) as *const _,
            );

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);

            (vao, vbo)
        };

        println!("[Ultralight] Initialized successfully (texture ID: {})", gl_texture);

        Ok(Self {
            lib,
            renderer,
            views: HashMap::new(),
            render_order: Vec::new(),
            focused_view: None,
            input_adapter: InputAdapter::new(),
            js_bridge: JsBridge::new(),
            want_capture_mouse: false,
            want_capture_keyboard: false,
            initialized: true,
            gl_texture,
            texture_size: (width, height),
            shader,
            vao,
            vbo,
        })
    }

    /// Check if the manager is properly initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Create a new view of the specified type.
    ///
    /// For now, all views are created as CPU-rendered (non-accelerated).
    /// GPU rendering will be implemented in Phase 2.
    pub fn create_view(
        &mut self,
        view_type: ViewType,
        width: u32,
        height: u32,
    ) -> Result<(), UltralightError> {
        if self.views.contains_key(&view_type) {
            return Err(UltralightError::ViewCreationFailed(format!(
                "View {:?} already exists",
                view_type
            )));
        }

        // Create view config for CPU rendering (non-accelerated)
        let view_config = UlViewConfig::start()
            .initial_device_scale(1.0)
            .font_family_standard("Arial")
            .is_accelerated(false) // CPU rendering
            .is_transparent(true)
            .build(self.lib.clone())
            .ok_or_else(|| UltralightError::ViewCreationFailed("Failed to create view config".to_string()))?;

        // Create the actual Ultralight view
        let ul_view = self.renderer
            .create_view(width, height, &view_config, None)
            .ok_or_else(|| UltralightError::ViewCreationFailed("Failed to create view".to_string()))?;

        // Note: We use a polling approach instead of callbacks for JS→Rust communication
        // JavaScript stores events in window.__rustPendingEvents, and we poll via drainEvents()

        // Wrap in our UltralightView struct
        let view = UltralightView::new(view_type, width, height, ul_view)?;

        println!(
            "[Ultralight] Created {:?} view ({}x{}, CPU rendered)",
            view_type,
            width,
            height,
        );

        self.views.insert(view_type, view);
        Ok(())
    }

    /// Load a URL into a view.
    pub fn load_url(&mut self, view_type: ViewType, url: &str) -> Result<(), UltralightError> {
        let view = self
            .views
            .get_mut(&view_type)
            .ok_or(UltralightError::ViewNotFound(view_type))?;
        view.load_url(url)
    }

    /// Load HTML content into a view.
    pub fn load_html(&mut self, view_type: ViewType, html: &str) -> Result<(), UltralightError> {
        let view = self
            .views
            .get_mut(&view_type)
            .ok_or(UltralightError::ViewNotFound(view_type))?;
        view.load_html(html)
    }

    /// Show a view and add it to the render order.
    pub fn show_view(&mut self, view_type: ViewType) -> Result<(), UltralightError> {
        let view = self
            .views
            .get_mut(&view_type)
            .ok_or(UltralightError::ViewNotFound(view_type))?;
        view.show();

        // Add to render order if not already present
        if !self.render_order.contains(&view_type) {
            self.render_order.push(view_type);
            self.sort_render_order();
        }

        println!("[Ultralight] Showing view: {:?}", view_type);
        Ok(())
    }

    /// Hide a view and remove it from the render order.
    pub fn hide_view(&mut self, view_type: ViewType) -> Result<(), UltralightError> {
        let view = self
            .views
            .get_mut(&view_type)
            .ok_or(UltralightError::ViewNotFound(view_type))?;
        view.hide();

        self.render_order.retain(|&vt| vt != view_type);

        if self.focused_view == Some(view_type) {
            self.focused_view = None;
        }

        println!("[Ultralight] Hiding view: {:?}", view_type);
        Ok(())
    }

    /// Toggle view visibility.
    pub fn toggle_view(&mut self, view_type: ViewType) -> Result<(), UltralightError> {
        let is_visible = self
            .views
            .get(&view_type)
            .ok_or(UltralightError::ViewNotFound(view_type))?
            .visible;

        if is_visible {
            self.hide_view(view_type)
        } else {
            self.show_view(view_type)
        }
    }

    /// Set focus to a view.
    pub fn focus_view(&mut self, view_type: ViewType) -> Result<(), UltralightError> {
        // Unfocus current view
        if let Some(current) = self.focused_view {
            if let Some(view) = self.views.get_mut(&current) {
                view.unfocus();
            }
        }

        // Focus new view
        let view = self
            .views
            .get_mut(&view_type)
            .ok_or(UltralightError::ViewNotFound(view_type))?;
        view.focus();
        self.focused_view = Some(view_type);

        Ok(())
    }

    /// Sort render order by z-index.
    fn sort_render_order(&mut self) {
        self.render_order.sort_by(|a, b| {
            let z_a = self.views.get(a).map(|v| v.z_index).unwrap_or(0);
            let z_b = self.views.get(b).map(|v| v.z_index).unwrap_or(0);
            z_a.cmp(&z_b)
        });
    }

    /// Handle a winit WindowEvent.
    ///
    /// Returns true if the event was consumed by a UI view.
    pub fn handle_event(&mut self, event: &WindowEvent) -> bool {
        let input_events = self.input_adapter.process_event(event);

        if input_events.is_empty() {
            return false;
        }

        let mouse_pos = self.input_adapter.mouse_position();

        // Find the topmost visible view under the cursor (for mouse events)
        let mut target_view: Option<ViewType> = None;
        for &view_type in self.render_order.iter().rev() {
            if let Some(view) = self.views.get(&view_type) {
                if view.contains_point(mouse_pos.0 as i32, mouse_pos.1 as i32) {
                    target_view = Some(view_type);
                    break;
                }
            }
        }

        // Update capture flags - check if mouse is over an actual UI element (not transparent area)
        self.want_capture_mouse = if let Some(view_type) = target_view {
            // Use JS to check if there's an actual UI element under the cursor
            self.is_mouse_over_ui_element(view_type, mouse_pos.0 as i32, mouse_pos.1 as i32)
        } else {
            false
        };
        self.want_capture_keyboard = self.focused_view.is_some();

        // Dispatch events to appropriate views
        for input_event in input_events {
            match &input_event {
                UltralightInputEvent::MouseMoved { .. }
                | UltralightInputEvent::MouseDown { .. }
                | UltralightInputEvent::MouseUp { .. }
                | UltralightInputEvent::Scroll { .. } => {
                    if let Some(view_type) = target_view {
                        self.dispatch_input_to_view(view_type, &input_event);
                    }
                }
                UltralightInputEvent::KeyDown { .. }
                | UltralightInputEvent::KeyUp { .. }
                | UltralightInputEvent::TextInput { .. } => {
                    if let Some(view_type) = self.focused_view {
                        self.dispatch_input_to_view(view_type, &input_event);
                    }
                }
            }
        }

        self.want_capture_mouse
    }

    /// Dispatch an input event to a specific view.
    fn dispatch_input_to_view(&mut self, view_type: ViewType, event: &UltralightInputEvent) {
        // Get the view's screen rect for coordinate transformation
        let screen_rect = if let Some(view) = self.views.get(&view_type) {
            view.get_screen_rect()
        } else {
            return;
        };

        match event {
            UltralightInputEvent::MouseMoved { x, y } => {
                // Transform screen coordinates to view-local coordinates
                let local_x = *x - screen_rect.0;
                let local_y = *y - screen_rect.1;

                if let Ok(mouse_event) = MouseEvent::new(
                    self.lib.clone(),
                    MouseEventType::MouseMoved,
                    local_x,
                    local_y,
                    UlMouseButton::None,
                ) {
                    if let Some(view) = self.views.get(&view_type) {
                        view.fire_mouse_event(mouse_event);
                    }
                }
            }
            UltralightInputEvent::MouseDown { x, y, button } => {
                // Focus this view when clicking on it
                self.focused_view = Some(view_type);

                let local_x = *x - screen_rect.0;
                let local_y = *y - screen_rect.1;
                let ul_button = match button {
                    UltralightMouseButton::Left => UlMouseButton::Left,
                    UltralightMouseButton::Right => UlMouseButton::Right,
                    UltralightMouseButton::Middle => UlMouseButton::Middle,
                };

                if let Ok(mouse_event) = MouseEvent::new(
                    self.lib.clone(),
                    MouseEventType::MouseDown,
                    local_x,
                    local_y,
                    ul_button,
                ) {
                    if let Some(view) = self.views.get(&view_type) {
                        view.fire_mouse_event(mouse_event);
                    }
                }

                // Focus the view on mouse down
                let _ = self.focus_view(view_type);
            }
            UltralightInputEvent::MouseUp { x, y, button } => {
                let local_x = *x - screen_rect.0;
                let local_y = *y - screen_rect.1;
                let ul_button = match button {
                    UltralightMouseButton::Left => UlMouseButton::Left,
                    UltralightMouseButton::Right => UlMouseButton::Right,
                    UltralightMouseButton::Middle => UlMouseButton::Middle,
                };

                if let Ok(mouse_event) = MouseEvent::new(
                    self.lib.clone(),
                    MouseEventType::MouseUp,
                    local_x,
                    local_y,
                    ul_button,
                ) {
                    if let Some(view) = self.views.get(&view_type) {
                        view.fire_mouse_event(mouse_event);
                    }
                }
            }
            UltralightInputEvent::Scroll { dx, dy } => {
                if let Ok(scroll_event) = ScrollEvent::new(
                    self.lib.clone(),
                    ScrollEventType::ScrollByPixel,
                    *dx,
                    *dy,
                ) {
                    if let Some(view) = self.views.get(&view_type) {
                        view.fire_scroll_event(scroll_event);
                    }
                }
            }
            UltralightInputEvent::KeyDown { key, modifiers } => {
                let vk = Self::map_vk_code(*key);
                let creation_info = KeyEventCreationInfo {
                    ty: KeyEventType::RawKeyDown,
                    modifiers: KeyEventModifiers {
                        alt: modifiers.alt,
                        ctrl: modifiers.ctrl,
                        meta: modifiers.meta,
                        shift: modifiers.shift,
                    },
                    virtual_key_code: vk,
                    native_key_code: *key as i32,
                    text: "",
                    unmodified_text: "",
                    is_keypad: false,
                    is_auto_repeat: false,
                    is_system_key: false,
                };
                if let Ok(key_event) = KeyEvent::new(self.lib.clone(), creation_info) {
                    if let Some(view) = self.views.get(&view_type) {
                        view.fire_key_event(key_event);
                    }
                }
            }
            UltralightInputEvent::KeyUp { key, modifiers } => {
                let vk = Self::map_vk_code(*key);
                let creation_info = KeyEventCreationInfo {
                    ty: KeyEventType::KeyUp,
                    modifiers: KeyEventModifiers {
                        alt: modifiers.alt,
                        ctrl: modifiers.ctrl,
                        meta: modifiers.meta,
                        shift: modifiers.shift,
                    },
                    virtual_key_code: vk,
                    native_key_code: *key as i32,
                    text: "",
                    unmodified_text: "",
                    is_keypad: false,
                    is_auto_repeat: false,
                    is_system_key: false,
                };
                if let Ok(key_event) = KeyEvent::new(self.lib.clone(), creation_info) {
                    if let Some(view) = self.views.get(&view_type) {
                        view.fire_key_event(key_event);
                    }
                }
            }
            UltralightInputEvent::TextInput { character } => {
                let text = character.to_string();
                let creation_info = KeyEventCreationInfo {
                    ty: KeyEventType::Char,
                    modifiers: KeyEventModifiers {
                        alt: false,
                        ctrl: false,
                        meta: false,
                        shift: false,
                    },
                    virtual_key_code: VirtualKeyCode::Unknown,
                    native_key_code: 0,
                    text: &text,
                    unmodified_text: &text,
                    is_keypad: false,
                    is_auto_repeat: false,
                    is_system_key: false,
                };
                if let Ok(key_event) = KeyEvent::new(self.lib.clone(), creation_info) {
                    if let Some(view) = self.views.get(&view_type) {
                        view.fire_key_event(key_event);
                    }
                }
            }
        }
    }

    /// Check if the mouse is over an actual UI element (not transparent/empty area).
    fn is_mouse_over_ui_element(&mut self, view_type: ViewType, x: i32, y: i32) -> bool {
        if let Some(view) = self.views.get_mut(&view_type) {
            if !view.visible {
                return false;
            }

            // Get view-local coordinates
            let screen_rect = view.get_screen_rect();
            let local_x = x - screen_rect.0;
            let local_y = y - screen_rect.1;

            // Use elementFromPoint to check if there's a UI element under the cursor
            // Returns false if the element is body/html (transparent area)
            let js = format!(
                r#"(function() {{
                    var el = document.elementFromPoint({}, {});
                    if (!el) return 'false';
                    var tag = el.tagName.toLowerCase();
                    // If it's body or html, mouse is in transparent area
                    if (tag === 'body' || tag === 'html') return 'false';
                    return 'true';
                }})()"#,
                local_x, local_y
            );

            if let Ok(result) = view.execute_js(&js) {
                return result.trim().trim_matches('"') == "true";
            }
        }
        false
    }

    /// Map Windows virtual key code to Ultralight VirtualKeyCode.
    fn map_vk_code(key: u32) -> VirtualKeyCode {
        match key {
            0x08 => VirtualKeyCode::Back,
            0x09 => VirtualKeyCode::Tab,
            0x0D => VirtualKeyCode::Return,
            0x1B => VirtualKeyCode::Escape,
            0x20 => VirtualKeyCode::Space,
            0x25 => VirtualKeyCode::Left,
            0x26 => VirtualKeyCode::Up,
            0x27 => VirtualKeyCode::Right,
            0x28 => VirtualKeyCode::Down,
            0x2E => VirtualKeyCode::Delete,
            0x30 => VirtualKeyCode::Key0,
            0x31 => VirtualKeyCode::Key1,
            0x32 => VirtualKeyCode::Key2,
            0x33 => VirtualKeyCode::Key3,
            0x34 => VirtualKeyCode::Key4,
            0x35 => VirtualKeyCode::Key5,
            0x36 => VirtualKeyCode::Key6,
            0x37 => VirtualKeyCode::Key7,
            0x38 => VirtualKeyCode::Key8,
            0x39 => VirtualKeyCode::Key9,
            0xBE => VirtualKeyCode::OemPeriod,
            0xBD => VirtualKeyCode::OemMinus,
            _ => VirtualKeyCode::Unknown,
        }
    }

    /// Update Ultralight (call each frame).
    ///
    /// This processes pending network requests, resource loads, and JavaScript timers.
    pub fn update(&mut self, _game_state: &ExposedGameState, _dt: f32) {
        if !self.initialized {
            return;
        }

        // Update the Ultralight renderer - handles network, resources, JS timers
        self.renderer.update();
    }

    /// Update the editor UI with current game state.
    ///
    /// Call this each frame when the editor is visible to keep the UI in sync.
    pub fn update_editor_state(&mut self, player_pos: [f32; 3], player_state: &str,
                                attack_state: &str, animation: &str) {
        if let Some(view) = self.views.get_mut(&ViewType::Editor) {
            if view.visible {
                // Build the JS call to update player data
                let js = format!(
                    r#"if(window.editorAPI && window.editorAPI.updatePlayerData) {{
                        window.editorAPI.updatePlayerData({{
                            position: [{}, {}, {}],
                            state: "{}",
                            attackState: "{}",
                            animation: "{}"
                        }});
                    }}"#,
                    player_pos[0], player_pos[1], player_pos[2],
                    player_state, attack_state, animation
                );
                let _ = view.execute_js(&js);
            }
        }
    }

    /// Update the editor dropdowns with entity types and factions.
    ///
    /// Call this when the editor is first shown or when entity types/factions change.
    pub fn update_editor_dropdowns(&mut self, entity_types: &[String], factions: &[String], emitter_types: &[String]) {
        if let Some(view) = self.views.get_mut(&ViewType::Editor) {
            if view.visible {
                // Convert to JSON arrays
                let entity_types_json = serde_json::to_string(entity_types).unwrap_or_else(|_| "[]".to_string());
                let factions_json = serde_json::to_string(factions).unwrap_or_else(|_| "[]".to_string());
                let emitter_types_json = serde_json::to_string(emitter_types).unwrap_or_else(|_| "[]".to_string());

                // Update entity types dropdown
                let js = format!(
                    r#"if(window.editorAPI && window.editorAPI.updateEntityTypes) {{
                        window.editorAPI.updateEntityTypes({});
                    }}"#,
                    entity_types_json
                );
                let _ = view.execute_js(&js);

                // Update factions dropdown
                let js = format!(
                    r#"if(window.editorAPI && window.editorAPI.updateFactions) {{
                        window.editorAPI.updateFactions({});
                    }}"#,
                    factions_json
                );
                let _ = view.execute_js(&js);

                // Update emitter types dropdown
                let js = format!(
                    r#"if(window.editorAPI && window.editorAPI.updateEmitterTypes) {{
                        window.editorAPI.updateEmitterTypes({});
                    }}"#,
                    emitter_types_json
                );
                let _ = view.execute_js(&js);
            }
        }
    }

    /// Update the emitter position in the editor.
    ///
    /// Call this when the user clicks on terrain while editing emitters.
    pub fn update_emitter_position(&mut self, pos: [f32; 3]) {
        if let Some(view) = self.views.get_mut(&ViewType::Editor) {
            if view.visible {
                let js = format!(
                    r#"if(document.getElementById('emitter-pos')) {{
                        document.getElementById('emitter-pos').value = '{},{},{}';
                    }}"#,
                    pos[0], pos[1], pos[2]
                );
                let _ = view.execute_js(&js);
            }
        }
    }

    /// Drain all pending JS events by polling JavaScript from all visible views.
    ///
    /// Calls `drainEvents()` in JavaScript which returns a JSON array of pending events
    /// and clears the queue on the JS side.
    pub fn drain_js_events(&mut self) -> Vec<String> {
        let mut all_events = Vec::new();

        // Poll events from all visible views
        for view in self.views.values_mut() {
            if view.visible {
                // Call drainEvents() which returns JSON array of event strings
                match view.execute_js("drainEvents()") {
                    Ok(result) => {
                        if !result.is_empty() && result != "[]" {
                            println!("[Ultralight] drainEvents returned: {}", result);
                        }
                        // Parse the JSON array
                        match serde_json::from_str::<Vec<String>>(&result) {
                            Ok(events) => {
                                if !events.is_empty() {
                                    println!("[Ultralight] Parsed {} events", events.len());
                                }
                                all_events.extend(events);
                            }
                            Err(e) => {
                                if !result.is_empty() {
                                    eprintln!("[Ultralight] Failed to parse events: {} - raw: {}", e, result);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[Ultralight] Failed to execute drainEvents(): {:?}", e);
                    }
                }
            }
        }
        all_events
    }

    /// Render all visible views.
    ///
    /// This renders dirty views to their surfaces and uploads to OpenGL texture.
    pub fn render(&mut self, fb_width: u32, fb_height: u32) {
        if !self.initialized || self.render_order.is_empty() {
            return;
        }

        // Render all dirty views to their surfaces
        self.renderer.render();

        // For each visible view, get the surface pixels and render to screen
        for &view_type in &self.render_order.clone() {
            if let Some(view) = self.views.get_mut(&view_type) {
                if !view.visible {
                    continue;
                }

                // Get the surface from the view and upload to texture
                if let Some(pixels) = view.get_surface_pixels() {
                    let (vx, vy, vw, vh) = view.screen_rect;

                    // Upload pixels to OpenGL texture
                    unsafe {
                        gl::BindTexture(gl::TEXTURE_2D, self.gl_texture);

                        // Resize texture if needed
                        if self.texture_size != (vw, vh) {
                            gl::TexImage2D(
                                gl::TEXTURE_2D,
                                0,
                                gl::RGBA8 as i32,
                                vw as i32,
                                vh as i32,
                                0,
                                gl::BGRA,
                                gl::UNSIGNED_BYTE,
                                std::ptr::null(),
                            );
                            self.texture_size = (vw, vh);
                        }

                        // Upload pixel data
                        gl::TexSubImage2D(
                            gl::TEXTURE_2D,
                            0,
                            0,
                            0,
                            vw as i32,
                            vh as i32,
                            gl::BGRA,
                            gl::UNSIGNED_BYTE,
                            pixels.as_ptr() as *const _,
                        );

                        gl::BindTexture(gl::TEXTURE_2D, 0);
                    }

                    // Render the texture as a fullscreen quad
                    self.render_textured_quad(vx, vy, vw, vh, fb_width, fb_height);
                }
            }
        }
    }

    /// Render a textured quad at the specified screen position.
    fn render_textured_quad(&self, x: i32, y: i32, w: u32, h: u32, fb_width: u32, fb_height: u32) {
        unsafe {
            // Save OpenGL state
            let mut blend_enabled: i32 = 0;
            let mut depth_test_enabled: i32 = 0;
            gl::GetIntegerv(gl::BLEND, &mut blend_enabled);
            gl::GetIntegerv(gl::DEPTH_TEST, &mut depth_test_enabled);

            // Set up for 2D rendering
            gl::Enable(gl::BLEND);
            // Use premultiplied alpha blending (Ultralight outputs premultiplied alpha)
            gl::BlendFunc(gl::ONE, gl::ONE_MINUS_SRC_ALPHA);
            gl::Disable(gl::DEPTH_TEST);

            // Activate shader
            self.shader.activate();

            // Bind texture
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.gl_texture);

            // Calculate normalized device coordinates
            let x0 = (x as f32 / fb_width as f32) * 2.0 - 1.0;
            let y0 = 1.0 - (y as f32 / fb_height as f32) * 2.0;
            let x1 = ((x as u32 + w) as f32 / fb_width as f32) * 2.0 - 1.0;
            let y1 = 1.0 - ((y as u32 + h) as f32 / fb_height as f32) * 2.0;

            // Build quad vertices (position + texcoord)
            // Note: y0 is top, y1 is bottom in NDC after our conversion
            #[rustfmt::skip]
            let vertices: [f32; 30] = [
                // Triangle 1: top-left, bottom-left, bottom-right
                x0, y0, 0.0,  0.0, 0.0,  // top-left
                x0, y1, 0.0,  0.0, 1.0,  // bottom-left
                x1, y1, 0.0,  1.0, 1.0,  // bottom-right
                // Triangle 2: top-left, bottom-right, top-right
                x0, y0, 0.0,  0.0, 0.0,  // top-left
                x1, y1, 0.0,  1.0, 1.0,  // bottom-right
                x1, y0, 0.0,  1.0, 0.0,  // top-right
            ];

            // Upload vertex data
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BufferSubData(
                gl::ARRAY_BUFFER,
                0,
                (vertices.len() * std::mem::size_of::<f32>()) as isize,
                vertices.as_ptr() as *const _,
            );
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);

            // Draw the quad
            gl::BindVertexArray(self.vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
            gl::BindVertexArray(0);

            gl::BindTexture(gl::TEXTURE_2D, 0);

            // Restore OpenGL state
            if blend_enabled == 0 {
                gl::Disable(gl::BLEND);
            }
            if depth_test_enabled != 0 {
                gl::Enable(gl::DEPTH_TEST);
            }
        }
    }

    /// Get a mutable reference to the JS bridge.
    pub fn js_bridge_mut(&mut self) -> &mut JsBridge {
        &mut self.js_bridge
    }

    /// Get a reference to the JS bridge.
    pub fn js_bridge(&self) -> &JsBridge {
        &self.js_bridge
    }

    /// Send a game event to be dispatched to JS views.
    pub fn send_game_event(&mut self, event: GameEvent) {
        self.js_bridge.send_event(event);
    }

    /// Execute JavaScript in a specific view.
    pub fn execute_js(
        &mut self,
        view_type: ViewType,
        script: &str,
    ) -> Result<String, UltralightError> {
        let view = self
            .views
            .get_mut(&view_type)
            .ok_or(UltralightError::ViewNotFound(view_type))?;
        view.execute_js(script)
    }

    /// Resize a view.
    pub fn resize_view(
        &mut self,
        view_type: ViewType,
        width: u32,
        height: u32,
    ) -> Result<(), UltralightError> {
        let view = self
            .views
            .get_mut(&view_type)
            .ok_or(UltralightError::ViewNotFound(view_type))?;
        view.resize(width, height);
        Ok(())
    }

    /// Handle window resize - resize all fullscreen views.
    pub fn handle_window_resize(&mut self, width: u32, height: u32) {
        for view in self.views.values_mut() {
            // Only resize views that are meant to be fullscreen
            if matches!(
                view.view_type,
                ViewType::Hud | ViewType::MainMenu | ViewType::PauseMenu | ViewType::LoadingScreen | ViewType::Editor
            ) {
                view.resize(width, height);
            }
        }
        // Update the stored texture size
        self.texture_size = (width, height);
    }
}

