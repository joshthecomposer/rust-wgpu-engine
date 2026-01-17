use crate::ui::game_new::render::UiGlyph;
use glyph_brush::{
    ab_glyph::{FontArc, PxScale},
    FontId, GlyphBrush, GlyphBrushBuilder, GlyphCruncher, Section, Text,
};
use std::collections::HashMap;
use std::fs;

pub struct FontSystem {
    pub(crate) glyph_brush: GlyphBrush<UiGlyph>,
    font_map: HashMap<String, FontId>,
}

impl FontSystem {
    pub fn new() -> Self {
        let font_paths = [
            ("resources/fonts/weiholmir.ttf", "Weiholmir"),
            ("resources/fonts/JetBrainsMono-Medium.ttf", "JetBrains Mono"),
            ("resources/fonts/OpenDyslexic-Regular.otf", "OpenDyslexic"),
        ];

        let mut fonts = Vec::new();
        for (path, name) in &font_paths {
            let font_data =
                fs::read(path).unwrap_or_else(|_| panic!("Failed to load font: {}", path));
            let font = FontArc::try_from_vec(font_data)
                .unwrap_or_else(|_| panic!("Error parsing font: {}", name));
            fonts.push(font);
        }

        let glyph_brush = GlyphBrushBuilder::using_fonts(fonts).build();

        let mut font_map = HashMap::new();
        font_map.insert("default".to_string(), FontId(0));
        font_map.insert("Weiholmir".to_string(), FontId(0));
        font_map.insert("weiholmir".to_string(), FontId(0));
        font_map.insert("JetBrains Mono".to_string(), FontId(1));
        font_map.insert("jetbrains mono".to_string(), FontId(1));
        font_map.insert("OpenDyslexic".to_string(), FontId(2));
        font_map.insert("opendyslexic".to_string(), FontId(2));

        Self {
            glyph_brush,
            font_map,
        }
    }

    pub fn get_font_id(&self, family_name: Option<&str>) -> FontId {
        match family_name {
            Some(name) => self
                .font_map
                .get(name)
                .or_else(|| self.font_map.get(&name.to_lowercase()))
                .copied()
                .unwrap_or(FontId(0)),
            None => FontId(0),
        }
    }

    /// Get font size multiplier for different fonts.
    /// Some fonts render smaller than others at the same size, so we scale them up.
    pub fn get_font_scale(&self, family_name: Option<&str>) -> f32 {
        match family_name {
            Some("OpenDyslexic") | Some("opendyslexic") => 1.6, // OpenDyslexic renders smaller
            Some("JetBrains Mono") | Some("jetbrains mono") => 1.6, // JetBrains Mono also renders smaller
            _ => 1.0,
        }
    }

    /// measures the dimensions of the given text with the specified font size
    pub fn measure_text(
        &mut self,
        text: &str,
        font_size: f32,
        font_family: Option<&str>,
    ) -> (f32, f32) {
        let font_id = self.get_font_id(font_family);
        let scale = self.get_font_scale(font_family);
        let scaled_size = font_size * scale;
        let section = Section {
            text: vec![Text::new(text)
                .with_scale(PxScale::from(scaled_size))
                .with_font_id(font_id)],
            ..Section::default()
        };

        if let Some(bounds) = self.glyph_brush.glyph_bounds(section) {
            (bounds.width() as f32, bounds.height() as f32)
        } else {
            (0.0, 0.0)
        }
    }
}

impl Default for FontSystem {
    fn default() -> Self {
        Self::new()
    }
}
