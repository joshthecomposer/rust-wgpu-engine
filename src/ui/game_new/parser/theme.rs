use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::assets;

use super::super::styles::{Color, Length};

#[derive(Debug, Clone, Deserialize)]
pub enum ThemeValue {
    Color(Color),
    Length(Length),
    String(String),
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Theme {
    #[serde(default)]
    pub colors: HashMap<String, Color>,
    // future expansion: generic properties map
    // pub properties: HashMap<String, ThemeValue>,
}

impl Theme {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_color(&self, name: &str) -> Option<Color> {
        self.colors.get(name).map(|c| c.clone())
    }
}

pub fn load_theme<P: AsRef<Path>>(path: P) -> Result<Theme, String> {
    let path = path
        .as_ref()
        .to_str()
        .ok_or_else(|| "Theme path contains invalid UTF-8".to_string())?;
    let content =
        assets::read_text(path).map_err(|e| format!("Failed to read theme file: {}", e))?;
    parse_theme(&content)
}

fn parse_theme(content: &str) -> Result<Theme, String> {
    ron::from_str(content).map_err(|e| format!("Failed to parse RON theme: {}", e))
}
