use glam::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    pub fn shrink(&self, amount: f32) -> Self {
        Self {
            x: self.x + amount,
            y: self.y + amount,
            width: (self.width - amount * 2.0).max(0.0),
            height: (self.height - amount * 2.0).max(0.0),
        }
    }

    pub fn shrink_by(&self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            x: self.x + left,
            y: self.y + top,
            width: (self.width - left - right).max(0.0),
            height: (self.height - top - bottom).max(0.0),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Length {
    Px(f32),
    Percent(f32),
    Auto,
    Variable(String),
}

impl Default for Length {
    fn default() -> Self {
        Length::Auto
    }
}

impl Length {
    pub fn resolve(&self, parent_size: f32) -> Option<f32> {
        match self {
            Length::Px(px) => Some(*px),
            Length::Percent(pct) => Some(parent_size * pct / 100.0),
            Length::Auto => None,
            Length::Variable(_) => None, // Variables should be resolved before layout
        }
    }

    pub fn resolve_or(&self, parent_size: f32, default: f32) -> f32 {
        self.resolve(parent_size).unwrap_or(default)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Color {
    Rgba(f32, f32, f32, f32),
    Hex(String),
    Variable(String),
}

impl Color {
    pub fn transparent() -> Self {
        Color::Rgba(0.0, 0.0, 0.0, 0.0)
    }

    pub fn white() -> Self {
        Color::Rgba(1.0, 1.0, 1.0, 1.0)
    }

    pub fn black() -> Self {
        Color::Rgba(0.0, 0.0, 0.0, 1.0)
    }

    pub fn red() -> Self {
        Color::Rgba(1.0, 0.0, 0.0, 1.0)
    }

    pub fn green() -> Self {
        Color::Rgba(0.0, 1.0, 0.0, 1.0)
    }

    pub fn blue() -> Self {
        Color::Rgba(0.0, 0.0, 1.0, 1.0)
    }

    pub fn to_rgba(&self) -> [f32; 4] {
        match self {
            Color::Rgba(r, g, b, a) => [*r, *g, *b, *a],
            Color::Hex(hex) => parse_hex_color(hex),
            Color::Variable(_) => [1.0, 0.0, 1.0, 1.0], // MAGENTA for unresolved variables
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::transparent()
    }
}

fn parse_hex_color(hex: &str) -> [f32; 4] {
    let hex = hex.trim_start_matches('#');
    let len = hex.len();

    if len == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
        [r, g, b, 1.0]
    } else if len == 8 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
        let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255) as f32 / 255.0;
        [r, g, b, a]
    } else {
        [0.0, 0.0, 0.0, 1.0]
    }
}

/// Configurable styling for scrollbar appearance.
///
/// All fields are optional - unspecified values use theme defaults.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ScrollbarStyle {
    /// Width of the scrollbar track in pixels.
    pub width: Option<f32>,
    /// Minimum height of the scrollbar thumb in pixels.
    pub thumb_min_height: Option<f32>,
    /// Background color of the scrollbar track.
    pub track_color: Option<Color>,
    /// Default color of the scrollbar thumb.
    pub thumb_color: Option<Color>,
    /// Color of the thumb when hovered.
    pub thumb_hover_color: Option<Color>,
    /// Color of the thumb when actively being dragged.
    pub thumb_active_color: Option<Color>,
}

impl ScrollbarStyle {
    /// Default scrollbar width.
    pub const DEFAULT_WIDTH: f32 = 10.0;
    /// Default minimum thumb height.
    pub const DEFAULT_THUMB_MIN_HEIGHT: f32 = 30.0;
    /// Default track color (stone-dark with alpha).
    pub const DEFAULT_TRACK_COLOR: [f32; 4] = [0.06, 0.09, 0.16, 0.5];
    /// Default thumb color (stone-mild).
    pub const DEFAULT_THUMB_COLOR: [f32; 4] = [0.4, 0.38, 0.33, 1.0];
    /// Default thumb hover color (stone-light).
    pub const DEFAULT_THUMB_HOVER_COLOR: [f32; 4] = [0.55, 0.53, 0.48, 1.0];
    /// Default thumb active color (runic-gold).
    pub const DEFAULT_THUMB_ACTIVE_COLOR: [f32; 4] = [0.85, 0.68, 0.42, 1.0];

    /// Returns the scrollbar width, using default if not specified.
    pub fn width(&self) -> f32 {
        self.width.unwrap_or(Self::DEFAULT_WIDTH)
    }

    /// Returns the minimum thumb height, using default if not specified.
    pub fn thumb_min_height(&self) -> f32 {
        self.thumb_min_height
            .unwrap_or(Self::DEFAULT_THUMB_MIN_HEIGHT)
    }

    /// Returns the track color, using default if not specified.
    pub fn track_color(&self) -> [f32; 4] {
        self.track_color
            .as_ref()
            .map(|c| c.to_rgba())
            .unwrap_or(Self::DEFAULT_TRACK_COLOR)
    }

    /// Returns the thumb color, using default if not specified.
    pub fn thumb_color(&self) -> [f32; 4] {
        self.thumb_color
            .as_ref()
            .map(|c| c.to_rgba())
            .unwrap_or(Self::DEFAULT_THUMB_COLOR)
    }

    /// Returns the thumb hover color, using default if not specified.
    pub fn thumb_hover_color(&self) -> [f32; 4] {
        self.thumb_hover_color
            .as_ref()
            .map(|c| c.to_rgba())
            .unwrap_or(Self::DEFAULT_THUMB_HOVER_COLOR)
    }

    /// Returns the thumb active color, using default if not specified.
    pub fn thumb_active_color(&self) -> [f32; 4] {
        self.thumb_active_color
            .as_ref()
            .map(|c| c.to_rgba())
            .unwrap_or(Self::DEFAULT_THUMB_ACTIVE_COLOR)
    }
}
