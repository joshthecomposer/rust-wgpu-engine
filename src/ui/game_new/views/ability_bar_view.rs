//! AbilityBarView - manages the ability bar UI in the custom GPU UI system.
//!
//! Replaces the Slint-based ability bar implementation.

use std::collections::HashMap;
use std::path::Path;

use image::GenericImageView;

use crate::renderer::{Renderer, UiTextureDescriptor};
use crate::ui::game::views::ability_bar::SlotDisplayData;
use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::parser::load_view_or_fallback;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::Rect;
use crate::ui::game_new::tree::UiTree;
use crate::ui::game_new::widgets::{AbilitySlot, TooltipManager, Widget};
use crate::ui::image_cache::UiImageCache;
use winit::keyboard::KeyCode;

/// Slot IDs corresponding to the RON layout.
const SLOT_IDS: [&str; 3] = ["slot_q", "slot_e", "slot_r"];

/// AbilityBarView manages the ability bar portion of the game HUD.
///
/// Position is at bottom-left of the screen.
pub struct AbilityBarView {
    tree: UiTree,
    tooltip: TooltipManager,
    needs_layout: bool,
    screen_width: f32,
    screen_height: f32,
    glow_time: f32,
    /// Cache of loaded ability icon textures (path -> texture_id)
    texture_cache: HashMap<String, u32>,
}

impl AbilityBarView {
    /// Create a new AbilityBarView.
    pub fn new(_font_system: &mut FontSystem) -> Self {
        let tree = load_view_or_fallback("resources/ui/ability_bar.ron");
        let tooltip = TooltipManager::new(Default::default());

        Self {
            tree,
            tooltip,
            needs_layout: true,
            screen_width: 1920.0,
            screen_height: 1080.0,
            glow_time: 0.0,
            texture_cache: HashMap::new(),
        }
    }

    /// Set the screen size for positioning.
    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        if self.screen_width != width || self.screen_height != height {
            self.screen_width = width;
            self.screen_height = height;
            self.needs_layout = true;
        }
    }

    /// Update ability bar data from game state.
    ///
    /// # Arguments
    /// * `slots` - Array of 6 slot display data (M1, M2, Q, E, Shift, R)
    /// * `delta_time` - Time since last frame for animations
    pub fn update_data(&mut self, slots: &[SlotDisplayData; 3], delta_time: f32) {
        self.glow_time += delta_time;
        if self.glow_time > 1000.0 {
            self.glow_time = 0.0;
        }

        for (i, slot_id) in SLOT_IDS.iter().enumerate() {
            let data = &slots[i];
            // Load texture from icon_path if not cached
            let texture_id = if !data.icon_path.is_empty() {
                self.load_or_get_texture(&data.icon_path)
            } else {
                0
            };

            if let Some(w) = self.tree.find_widget_mut(slot_id) {
                if let Some(slot) = w.as_any_mut().downcast_mut::<AbilitySlot>() {
                    slot.set_data(
                        texture_id,
                        data.cooldown_progress,
                        data.is_ready,
                        &data.ability_id,
                        &data.ability_name,
                        &data.ability_description,
                    );
                    slot.update_glow_time(delta_time);
                }
            }
        }

        self.needs_layout = true;
    }

    /// Load a texture from a file path, or return cached texture ID.
    fn load_or_get_texture(&mut self, path: &str) -> u32 {
        if let Some(&tex_id) = self.texture_cache.get(path) {
            return tex_id;
        }

        // Load new texture
        if !Path::new(path).exists() {
            return 0;
        }

        let tex_id = Self::load_texture_from_file(path);
        if tex_id > 0 {
            self.texture_cache.insert(path.to_string(), tex_id);
        }
        tex_id
    }

    /// Load an image file as an OpenGL texture.
    fn load_texture_from_file(path: &str) -> u32 {
        let img = match image::open(path) {
            Ok(img) => img,
            Err(_) => {
                eprintln!("[AbilityBarView] Failed to load icon: {}", path);
                return 0;
            }
        };

        let (width, height) = img.dimensions();
        let rgba = img.to_rgba8();
        let raw = rgba.as_raw();

        Renderer::create_ui_texture(
            UiTextureDescriptor::rgba8_linear_clamped(width, height),
            Some(raw),
        )
    }

    /// Process input and update tooltips.
    pub fn update(&mut self, ctx: &mut UiContext) {
        self.tooltip.begin_frame();
        self.tree.update(ctx);

        // Key press feedback (matches the "flash on Q/E/R press" feel)
        if ctx.input.just_pressed(KeyCode::KeyQ) {
            if let Some(w) = self.tree.find_widget_mut("slot_q") {
                if let Some(slot) = w.as_any_mut().downcast_mut::<AbilitySlot>() {
                    slot.trigger_use_flash();
                }
            }
        }
        if ctx.input.just_pressed(KeyCode::KeyE) {
            if let Some(w) = self.tree.find_widget_mut("slot_e") {
                if let Some(slot) = w.as_any_mut().downcast_mut::<AbilitySlot>() {
                    slot.trigger_use_flash();
                }
            }
        }
        if ctx.input.just_pressed(KeyCode::KeyR) {
            if let Some(w) = self.tree.find_widget_mut("slot_r") {
                if let Some(slot) = w.as_any_mut().downcast_mut::<AbilitySlot>() {
                    slot.trigger_use_flash();
                }
            }
        }

        // Check each slot for hover and show tooltip
        for slot_id in &SLOT_IDS {
            if let Some(w) = self.tree.find_widget_mut(slot_id) {
                let rect = w.rect();
                if let Some(slot) = w.as_any_mut().downcast_mut::<AbilitySlot>() {
                    if let Some((name, desc)) = slot.get_tooltip_info() {
                        self.tooltip.show(name, desc, rect);
                    }
                }
            }
        }
    }

    /// Layout the ability bar at the bottom-left of the screen.
    pub fn layout(&mut self, font_system: &mut FontSystem) {
        const MARGIN: f32 = 10.0;
        const BAR_WIDTH: f32 = 172.0;
        const BAR_HEIGHT: f32 = 68.0;

        // Size the tree to the bar dimensions
        self.tree.set_screen_size(BAR_WIDTH, BAR_HEIGHT);

        // Position at bottom-left with margin
        let offset_x = MARGIN;
        let offset_y = self.screen_height - BAR_HEIGHT - MARGIN;
        self.tree.set_offset(offset_x, offset_y);

        self.tree.force_layout();
        self.tree.layout(font_system);

        // Layout tooltip overlay (full screen so it can appear anywhere)
        let full_screen = Rect::new(0.0, 0.0, self.screen_width, self.screen_height);
        self.tooltip.layout(font_system, full_screen);
    }

    /// Render the ability bar.
    pub fn render(&self, renderer: &mut UiRenderer) {
        self.tree.render(renderer);
        self.tooltip.render(renderer);
    }

    /// Returns true if layout needs to be recalculated.
    pub fn needs_layout(&self) -> bool {
        self.needs_layout
    }

    /// Clears the layout flag after rendering.
    pub fn clear_layout_flag(&mut self) {
        self.needs_layout = false;
    }
}

/// Helper function to convert SlotDisplayData to texture ID.
/// Uses UiImageCache to load textures from icon paths.
pub fn load_slot_textures(
    slots: &[SlotDisplayData; 6],
    _image_cache: &mut UiImageCache,
) -> [u32; 6] {
    let mut textures = [0u32; 6];
    for (i, slot) in slots.iter().enumerate() {
        if slot.visible && !slot.icon_path.is_empty() {
            // UiImageCache returns slint::Image, but we need OpenGL texture ID.
            // This is a compatibility bridge - we'll need to adapt based on actual image loading.
            // For now, we assume texture_id is passed in separately or we use a fallback.
            // TODO: Implement proper texture loading from icon_path
            textures[i] = 0; // Placeholder - will be loaded externally
        }
    }
    textures
}
