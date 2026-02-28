use serde::{Deserialize, Serialize};

use super::types::{Color, Length};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Style {
    #[serde(default)]
    pub id: Option<String>,

    #[serde(default)]
    pub width: Length,
    #[serde(default)]
    pub height: Length,

    #[serde(default)]
    pub min_width: Length,
    #[serde(default)]
    pub min_height: Length,
    #[serde(default)]
    pub max_width: Length,
    #[serde(default)]
    pub max_height: Length,

    #[serde(default)]
    pub margin: Length,
    #[serde(default)]
    pub margin_top: Option<Length>,
    #[serde(default)]
    pub margin_right: Option<Length>,
    #[serde(default)]
    pub margin_bottom: Option<Length>,
    #[serde(default)]
    pub margin_left: Option<Length>,

    #[serde(default)]
    pub padding: Length,
    #[serde(default)]
    pub padding_top: Option<Length>,
    #[serde(default)]
    pub padding_right: Option<Length>,
    #[serde(default)]
    pub padding_bottom: Option<Length>,
    #[serde(default)]
    pub padding_left: Option<Length>,

    #[serde(default)]
    pub background: Color,
    // text color
    #[serde(default)]
    pub color: Option<Color>,

    #[serde(default)]
    pub font_size: Option<f32>,

    #[serde(default)]
    pub font_family: Option<String>,

    #[serde(default)]
    pub text_align: Option<super::layout::Alignment>,

    #[serde(default)]
    pub border_width: f32,
    #[serde(default)]
    pub border_color: Color,
    #[serde(default)]
    pub border_radius: f32,
}

impl Style {
    pub fn margin_top(&self) -> Length {
        self.margin_top.clone().unwrap_or(self.margin.clone())
    }

    pub fn margin_right(&self) -> Length {
        self.margin_right.clone().unwrap_or(self.margin.clone())
    }

    pub fn margin_bottom(&self) -> Length {
        self.margin_bottom.clone().unwrap_or(self.margin.clone())
    }

    pub fn margin_left(&self) -> Length {
        self.margin_left.clone().unwrap_or(self.margin.clone())
    }

    pub fn padding_top(&self) -> Length {
        self.padding_top.clone().unwrap_or(self.padding.clone())
    }

    pub fn padding_right(&self) -> Length {
        self.padding_right.clone().unwrap_or(self.padding.clone())
    }

    pub fn padding_bottom(&self) -> Length {
        self.padding_bottom.clone().unwrap_or(self.padding.clone())
    }

    pub fn padding_left(&self) -> Length {
        self.padding_left.clone().unwrap_or(self.padding.clone())
    }

    pub fn resolve_margins(&self, parent_width: f32, parent_height: f32) -> (f32, f32, f32, f32) {
        (
            self.margin_top().resolve_or(parent_height, 0.0),
            self.margin_right().resolve_or(parent_width, 0.0),
            self.margin_bottom().resolve_or(parent_height, 0.0),
            self.margin_left().resolve_or(parent_width, 0.0),
        )
    }

    pub fn resolve_padding(&self, parent_width: f32, parent_height: f32) -> (f32, f32, f32, f32) {
        (
            self.padding_top().resolve_or(parent_height, 0.0),
            self.padding_right().resolve_or(parent_width, 0.0),
            self.padding_bottom().resolve_or(parent_height, 0.0),
            self.padding_left().resolve_or(parent_width, 0.0),
        )
    }
}
