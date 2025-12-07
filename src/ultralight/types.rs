//! Core types for the Ultralight integration.

#![allow(dead_code)]

/// Error type for Ultralight operations.
#[derive(Debug)]
pub enum UltralightError {
    /// Failed to initialize Ultralight renderer
    InitializationFailed(String),
    /// Failed to create a view
    ViewCreationFailed(String),
    /// Failed to load URL or HTML content
    LoadFailed(String),
    /// View not found
    ViewNotFound(ViewType),
    /// JavaScript execution error
    JsError(String),
    /// GPU driver error
    GpuDriverError(String),
    /// File system error
    FileError(String),
}

impl std::fmt::Display for UltralightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UltralightError::InitializationFailed(msg) => {
                write!(f, "Ultralight initialization failed: {}", msg)
            }
            UltralightError::ViewCreationFailed(msg) => {
                write!(f, "View creation failed: {}", msg)
            }
            UltralightError::LoadFailed(msg) => write!(f, "Load failed: {}", msg),
            UltralightError::ViewNotFound(vt) => write!(f, "View not found: {:?}", vt),
            UltralightError::JsError(msg) => write!(f, "JavaScript error: {}", msg),
            UltralightError::GpuDriverError(msg) => write!(f, "GPU driver error: {}", msg),
            UltralightError::FileError(msg) => write!(f, "File error: {}", msg),
        }
    }
}

impl std::error::Error for UltralightError {}

/// Types of UI views that can be created.
///
/// GPU-rendered views are used for performance-critical game UI,
/// while CPU-rendered views are used for editor/debug tools where
/// flexibility is more important than raw performance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewType {
    // === GPU Rendered (Performance Critical) ===
    /// In-game HUD (health, mana, minimap, etc.)
    Hud,
    /// Player inventory screen
    Inventory,
    /// NPC dialogue interface
    Dialogue,
    /// Pause menu
    PauseMenu,
    /// Main menu / title screen
    MainMenu,
    /// Loading screen
    LoadingScreen,

    // === CPU Rendered (Editor/Debug) ===
    /// Main editor panel (replaces ImGui editor)
    Editor,
    /// Entity inspector panel
    EntityInspector,
    /// Debug console
    Console,
    /// Performance profiler
    Profiler,
    /// Asset browser
    AssetBrowser,
    /// World editor
    WorldEditor,
}

impl ViewType {
    /// Returns true if this view type should use GPU rendering.
    pub fn is_gpu_rendered(&self) -> bool {
        matches!(
            self,
            ViewType::Hud
                | ViewType::Inventory
                | ViewType::Dialogue
                | ViewType::PauseMenu
                | ViewType::MainMenu
                | ViewType::LoadingScreen
        )
    }

    /// Returns the default HTML file path for this view type.
    pub fn default_html_path(&self) -> &'static str {
        match self {
            ViewType::Hud => "resources/ui/hud.html",
            ViewType::Inventory => "resources/ui/inventory.html",
            ViewType::Dialogue => "resources/ui/dialogue.html",
            ViewType::PauseMenu => "resources/ui/pause_menu.html",
            ViewType::MainMenu => "resources/ui/main_menu.html",
            ViewType::LoadingScreen => "resources/ui/loading.html",
            ViewType::Editor => "resources/ui/editor.html",
            ViewType::EntityInspector => "resources/ui/entity_inspector.html",
            ViewType::Console => "resources/ui/console.html",
            ViewType::Profiler => "resources/ui/profiler.html",
            ViewType::AssetBrowser => "resources/ui/asset_browser.html",
            ViewType::WorldEditor => "resources/ui/world_editor.html",
        }
    }

    /// Returns the default z-index for this view type.
    /// Higher values render on top.
    pub fn default_z_index(&self) -> i32 {
        match self {
            ViewType::Hud => 100,
            ViewType::Inventory => 200,
            ViewType::Dialogue => 150,
            ViewType::PauseMenu => 300,
            ViewType::MainMenu => 400,
            ViewType::LoadingScreen => 500,
            ViewType::Editor => 80,
            ViewType::EntityInspector => 50,
            ViewType::Console => 60,
            ViewType::Profiler => 40,
            ViewType::AssetBrowser => 30,
            ViewType::WorldEditor => 20,
        }
    }
}

/// Configuration for creating a new Ultralight view.
#[derive(Debug, Clone)]
pub struct ViewConfig {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Whether to use GPU rendering (true) or CPU rendering (false)
    pub gpu_accelerated: bool,
    /// Whether the view has a transparent background
    pub transparent: bool,
    /// Initial visibility
    pub visible: bool,
    /// Z-index for rendering order (higher = on top)
    pub z_index: i32,
    /// Screen position (x, y) in pixels from top-left
    pub position: (i32, i32),
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            gpu_accelerated: true,
            transparent: true,
            visible: true,
            z_index: 0,
            position: (0, 0),
        }
    }
}

impl ViewConfig {
    /// Create a new ViewConfig with default values for the given ViewType.
    pub fn for_view_type(view_type: ViewType, width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            gpu_accelerated: view_type.is_gpu_rendered(),
            transparent: true,
            visible: false, // Views start hidden
            z_index: view_type.default_z_index(),
            position: (0, 0),
        }
    }
}

/// Events sent from JavaScript to Rust.
///
/// These are queued by the JsBridge and processed in the game loop.
#[derive(Debug, Clone)]
pub enum JsEvent {
    /// Button clicked in UI
    ButtonClicked { button_id: String },
    /// Inventory slot interaction
    InventoryAction {
        action: String,
        slot_id: u32,
        item_id: Option<u32>,
    },
    /// Menu option selected
    MenuSelect { menu_id: String, option: String },
    /// Dialogue choice made
    DialogueChoice { dialogue_id: String, choice_index: u32 },
    /// Custom event with arbitrary JSON data
    Custom { event_type: String, data: String },

    // === Editor Events ===
    /// Light settings changed
    LightUpdate {
        dir_x: f32,
        dir_y: f32,
        dir_z: f32,
        distance: f32,
        shadow_debug: bool,
        ortho_near: f32,
        ortho_far: f32,
        ortho_bounds: f32,
        bias_scalar: f32,
    },
    /// Volume changed
    VolumeChange { volume: f32 },
    /// Sound pause/play
    SoundControl { action: String },
    /// Save entity state
    SaveEntityState,
    /// Create faction
    CreateFaction { name: String },
    /// Toggle create mode for entity placement
    CreateModeToggle {
        enabled: bool,
        entity_type: String,
        faction: String,
        weapon: Option<String>,
        base_speed: f32,
    },
    /// Create new entity type
    CreateEntityType {
        entity_type: String,
        rot_correction: [f32; 4],
        scale_correction: [f32; 3],
        mesh_path: String,
        texture_path: String,
        aggro_range: f32,
        total_mass: f32,
        hitbox: String,
        radius: f32,
        height: f32,
        half_extents: [f32; 3],
    },
    /// Delete entity type
    DeleteEntityType { entity_type: String },
    /// Update entity position
    UpdateEntityPosition { entity_id: u32, position: [f32; 3] },
    /// Toggle emitter preview rendering
    RenderEmitterPreview { enabled: bool },
    /// Save emitter definition
    SaveEmitter { emitter_json: String },
}

/// Events/state sent from Rust to JavaScript.
///
/// These are used to update the UI with current game state.
#[derive(Debug, Clone)]
pub enum GameEvent {
    /// Update player stats display
    PlayerStatsUpdate {
        health: f32,
        max_health: f32,
        mana: f32,
        max_mana: f32,
        level: u32,
        experience: u32,
    },
    /// Update inventory display
    InventoryUpdate { items_json: String },
    /// Show dialogue
    ShowDialogue {
        speaker: String,
        text: String,
        choices_json: String,
    },
    /// Hide dialogue
    HideDialogue,
    /// Show notification/toast
    ShowNotification { message: String, duration_ms: u32 },
    /// Custom event with arbitrary JSON data
    Custom { event_type: String, data: String },
}

/// State exposed to JavaScript for reading.
#[derive(Debug, Clone, Default)]
pub struct ExposedGameState {
    /// Player health (0.0 - 1.0 normalized)
    pub player_health: f32,
    /// Player mana (0.0 - 1.0 normalized)
    pub player_mana: f32,
    /// Player level
    pub player_level: u32,
    /// Current gold/currency
    pub gold: u32,
    /// Current quest objectives as JSON
    pub quest_objectives_json: String,
    /// Minimap data as JSON
    pub minimap_json: String,
    /// Is game paused
    pub is_paused: bool,
    /// Current game time
    pub game_time: f32,
}

/// Editor-specific state exposed to JavaScript.
#[derive(Debug, Clone, Default)]
pub struct EditorState {
    /// Player position
    pub player_position: [f32; 3],
    /// Player state name
    pub player_state: String,
    /// Player attack state name
    pub player_attack_state: String,
    /// Current animation name
    pub player_animation: String,
    /// Available entity types
    pub entity_types: Vec<String>,
    /// Available factions
    pub factions: Vec<String>,
    /// Available emitter types
    pub emitter_types: Vec<String>,
    /// Currently selected entities (as JSON)
    pub selected_entities_json: String,
    /// Light settings
    pub light_dir: [f32; 3],
    pub light_distance: f32,
    pub shadow_debug: bool,
    pub ortho_near: f32,
    pub ortho_far: f32,
    pub ortho_bounds: f32,
    pub bias_scalar: f32,
    /// Master volume
    pub master_volume: f32,
}

