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
            Color::Variable(_) => [0.0, 0.0, 0.0, 1.0], // Should be resolved before rendering
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
