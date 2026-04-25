//! AbilitySlot widget - displays a single ability slot with icon, cooldown overlay, and key label.
//!
//! Used for building ability bars. Supports:
//! - Icon texture rendering (with fallback to ability ID text)
//! - Cooldown overlay (top-down darkening based on cooldown progress)
//! - Ready glow effect (golden border pulse when ability is ready)
//! - Key label in top-right corner

use std::f32::consts::PI;

use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{Color, Rect, Style};

use super::Widget;

/// Size of individual ability slots in pixels.
const SLOT_SIZE: f32 = 48.0;
/// Border width for ready glow effect.
const GLOW_BORDER_WIDTH: f32 = 2.0;
/// Font size for key label.
const KEY_LABEL_FONT_SIZE: f32 = 9.0;
/// Font size for ability ID fallback.
const ABILITY_ID_FONT_SIZE: f32 = 20.0;

/// A single ability slot widget displaying an icon, cooldown, and key label.
///
/// # Visual Structure
/// ```text
/// ┌─────────────────────┐
/// │ ┌─────┐ [key]       │  ← Key label in top-right
/// │ │icon │             │  ← Icon or ability ID fallback
/// │ └─────┘             │
/// │▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓│  ← Cooldown overlay (top-down)
/// └─────────────────────┘
/// ```
pub struct AbilitySlot {
    pub style: Style,
    /// Computed bounding rect after layout.
    rect: Rect,

    // Slot data (updated dynamically)
    /// The OpenGL texture ID for the ability icon. 0 means no icon.
    pub texture_id: u32,
    /// Key label to display (e.g., "M1", "Q", "Shift").
    pub key_label: String,
    /// Cooldown progress: 0.0 = ready, 1.0 = full cooldown.
    pub cooldown_progress: f32,
    /// Whether the ability is ready to use (triggers glow effect).
    pub is_ready: bool,
    /// Ability ID string, shown when no icon is available.
    pub ability_id: String,

    // Tooltip data
    /// Name of the ability for tooltip.
    pub ability_name: String,
    /// Description of the ability for tooltip.
    pub ability_description: String,

    // Colors (from theme or defaults)
    /// Background color of the slot.
    pub slot_background: Color,
    /// Border color when ready.
    pub ready_border_color: Color,
    /// Border color when not ready.
    pub normal_border_color: Color,

    // Animation state
    /// Time accumulator for glow animation (updated externally).
    pub glow_time: f32,
    /// Time remaining for the ready glow effect (decays to 0).
    glow_remaining: f32,
    /// Time remaining for a short "used/pressed" flash (decays to 0).
    use_flash_remaining: f32,
    /// Previous cooldown progress to detect when ability becomes ready.
    prev_cooldown_progress: f32,

    // Interaction state
    is_hovered: bool,
}

impl AbilitySlot {
    /// Creates a new AbilitySlot with default styling.
    pub fn new(style: Style) -> Self {
        Self {
            style,
            rect: Rect::default(),
            texture_id: 0,
            key_label: String::new(),
            cooldown_progress: 0.0,
            is_ready: false,
            ability_id: String::new(),
            ability_name: String::new(),
            ability_description: String::new(),
            slot_background: Color::Rgba(0.06, 0.09, 0.16, 0.9), // deep-void-alpha
            ready_border_color: Color::Rgba(0.85, 0.47, 0.02, 1.0), // runic-gold
            normal_border_color: Color::Rgba(0.28, 0.33, 0.42, 1.0), // stone-light
            glow_time: 0.0,
            glow_remaining: 0.0,
            use_flash_remaining: 0.0,
            prev_cooldown_progress: 0.0,
            is_hovered: false,
        }
    }

    /// Builder: set key label.
    pub fn with_key_label(mut self, label: impl Into<String>) -> Self {
        self.key_label = label.into();
        self
    }

    /// Builder: set slot background color.
    pub fn with_slot_background(mut self, color: Color) -> Self {
        self.slot_background = color;
        self
    }

    /// Builder: set ready border color.
    pub fn with_ready_border_color(mut self, color: Color) -> Self {
        self.ready_border_color = color;
        self
    }

    /// Builder: set normal border color.
    pub fn with_normal_border_color(mut self, color: Color) -> Self {
        self.normal_border_color = color;
        self
    }

    /// Update slot data from game state.
    pub fn set_data(
        &mut self,
        texture_id: u32,
        cooldown_progress: f32,
        is_ready: bool,
        ability_id: &str,
        ability_name: &str,
        ability_description: &str,
    ) {
        self.texture_id = texture_id;
        self.cooldown_progress = cooldown_progress;
        self.is_ready = is_ready;
        self.ability_id = ability_id.to_string();
        self.ability_name = ability_name.to_string();
        self.ability_description = ability_description.to_string();
    }

    /// Update the glow animation time and decay glow effect.
    pub fn update_glow_time(&mut self, delta: f32) {
        self.glow_time += delta;
        // Wrap around to prevent float overflow
        if self.glow_time > 1000.0 {
            self.glow_time = 0.0;
        }

        // Decay the glow remaining time
        if self.glow_remaining > 0.0 {
            self.glow_remaining = (self.glow_remaining - delta).max(0.0);
        }

        // Decay the use-flash time
        if self.use_flash_remaining > 0.0 {
            self.use_flash_remaining = (self.use_flash_remaining - delta).max(0.0);
        }

        // Detect when ability was just used (transition from ready to cooldown)
        if self.prev_cooldown_progress == 0.0 && self.cooldown_progress > 0.0 {
            // quick flash (tweak to taste)
            self.use_flash_remaining = 0.12;
        }

        // Detect when ability just came off cooldown (transition from cooldown to ready)
        if self.prev_cooldown_progress > 0.0 && self.cooldown_progress == 0.0 && self.is_ready {
            // Trigger glow for 1.5 seconds
            self.glow_remaining = 1.5;
            self.glow_time = 0.0; // Reset animation phase
        }
        self.prev_cooldown_progress = self.cooldown_progress;
    }

    /// Trigger a short visual flash (e.g. on key press/use).
    pub fn trigger_use_flash(&mut self) {
        self.use_flash_remaining = 0.25;
    }

    /// Returns true if this slot is being hovered.
    pub fn is_hovered(&self) -> bool {
        self.is_hovered
    }

    /// Returns tooltip info (name, description) if slot is hovered and has ability.
    pub fn get_tooltip_info(&self) -> Option<(&str, &str)> {
        if self.is_hovered && !self.ability_name.is_empty() {
            Some((&self.ability_name, &self.ability_description))
        } else {
            None
        }
    }
}

impl Widget for AbilitySlot {
    fn layout(&mut self, _font_system: &mut FontSystem, available: Rect) {
        // get margins from style
        let (margin_top, _margin_right, _margin_bottom, margin_left) = self
            .style
            .resolve_margins(available.width, available.height);

        // abilitySlot uses fixed size from style, or default SLOT_SIZE
        // don't clamp by available - parent should allocate enough space
        let content_width = self.style.width.resolve_or(available.width, SLOT_SIZE);
        let content_height = self.style.height.resolve_or(available.height, SLOT_SIZE);

        // position with margin offset
        self.rect = Rect::new(
            available.x + margin_left,
            available.y + margin_top,
            content_width,
            content_height,
        );
    }

    fn update(&mut self, ctx: &mut UiContext) -> bool {
        let mouse_pos = ctx.mouse_pos();
        self.is_hovered = self.rect.contains(mouse_pos);

        if self.is_hovered && ctx.is_click_start() {
            if let Some(id) = &self.style.id {
                println!("[AbilitySlot] Clicked. ID: {}", id);
            }
            return true;
        }
        false
    }

    fn render(&self, renderer: &mut UiRenderer) {
        let border_radius = self.style.border_radius;
        let border_width = if self.is_ready {
            GLOW_BORDER_WIDTH
        } else {
            1.0
        };

        let should_glow = self.glow_remaining > 0.0;
        let border_color = if should_glow {
            // pulsing glow effect using sine wave
            let pulse = (self.glow_time * 3.0 * PI).sin() * 0.3 + 0.7; // oscillate between 0.4 and 1.0
            let base = self.ready_border_color.to_rgba();
            [base[0] * pulse, base[1] * pulse, base[2] * pulse, base[3]]
        } else {
            self.normal_border_color.to_rgba()
        };

        // draw border (outer rect)
        renderer.draw_rect(self.rect, border_color, border_radius);

        // draw background (inner rect)
        let bg_rect = self.rect.shrink(border_width);
        let bg_color = self.slot_background.to_rgba();
        let inner_radius = (border_radius - border_width).max(0.0);
        if bg_rect.width > 0.0 && bg_rect.height > 0.0 {
            renderer.draw_rect(bg_rect, bg_color, inner_radius);
        }

        // draw ability icon or fallback ID
        if self.texture_id > 0 {
            // draw icon texture
            let icon_rect = bg_rect.shrink(2.0); // small inset for padding
            if icon_rect.width > 0.0 && icon_rect.height > 0.0 {
                // darken icon if not ready (on cooldown)
                let tint = if self.is_ready {
                    None
                } else {
                    Some([0.4, 0.4, 0.4, 1.0]) // darken when on cooldown
                };
                renderer.draw_textured_rect(icon_rect, self.texture_id, tint);
            }
        } else if !self.ability_id.is_empty() {
            // draw ability ID as fallback text (centered)
            let text_color = [0.58, 0.64, 0.72, 0.5]; // old-text with alpha
            let text_x = self.rect.x + self.rect.width / 2.0
                - (self.ability_id.len() as f32 * ABILITY_ID_FONT_SIZE * 0.3);
            let text_y = self.rect.y + self.rect.height / 2.0 - ABILITY_ID_FONT_SIZE / 2.0;
            renderer.draw_text(
                &self.ability_id,
                text_x,
                text_y,
                ABILITY_ID_FONT_SIZE,
                text_color,
                None,
            );
        }

        // draw cooldown overlay (from top down based on progress)
        if self.cooldown_progress > 0.0 {
            let overlay_height = bg_rect.height * self.cooldown_progress;
            let overlay_rect = Rect::new(bg_rect.x, bg_rect.y, bg_rect.width, overlay_height);
            let overlay_color = [0.0, 0.0, 0.0, 0.6];
            renderer.draw_rect(overlay_rect, overlay_color, inner_radius);
        }

        // short "used" flash overlay (match MenuButton hover accent color)
        if self.use_flash_remaining > 0.0 {
            let flash_color = [0.4, 0.38, 0.33, 1.0];
            renderer.draw_rect(bg_rect, flash_color, inner_radius);
        }

        // key label in top-right corner
        if !self.key_label.is_empty() {
            let label_padding = 3.0;
            let label_width =
                self.key_label.len() as f32 * KEY_LABEL_FONT_SIZE * 0.6 + label_padding * 2.0;
            let label_height = KEY_LABEL_FONT_SIZE + label_padding * 2.0;
            let label_x = self.rect.x + self.rect.width - label_width - 2.0;
            let label_y = self.rect.y + 2.0;

            let label_rect = Rect::new(label_x, label_y, label_width, label_height);
            let stone_color = Color::Rgba(0.12, 0.16, 0.23, 0.9);
            renderer.draw_rect(label_rect, stone_color.to_rgba(), 2.0);

            let text_color = [0.58, 0.64, 0.72, 1.0];
            let text_x = label_x + label_padding;
            let text_y = label_y + label_padding;
            renderer.draw_text(
                &self.key_label,
                text_x,
                text_y,
                KEY_LABEL_FONT_SIZE,
                text_color,
                None,
            );
        }

        if self.glow_remaining > 0.0 {
            let glow_offset = 2.0;
            let glow_rect = Rect::new(
                self.rect.x - glow_offset,
                self.rect.y - glow_offset,
                self.rect.width + glow_offset * 2.0,
                self.rect.height + glow_offset * 2.0,
            );
            let fade = (self.glow_remaining / 1.5).min(1.0);
            let pulse = (self.glow_time * 3.0 * PI).sin() * 0.25 + 0.25;
            let glow_color = Color::Rgba(0.85, 0.47, 0.02, pulse * fade);
            renderer.draw_rect(glow_rect, glow_color.to_rgba(), border_radius + glow_offset);
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn id(&self) -> Option<&str> {
        self.style.id.as_deref()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn find_widget_mut(&mut self, id: &str) -> Option<&mut dyn Widget> {
        if self.id() == Some(id) {
            return Some(self);
        }
        None
    }
}
