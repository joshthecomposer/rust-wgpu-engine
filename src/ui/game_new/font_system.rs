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
        // TODO: Move path configuration to a config or resource manager
        let font_path = "resources/fonts/weiholmir.ttf";
        let font_data =
            fs::read(font_path).unwrap_or_else(|_| panic!("Failed to load font: {}", font_path));
        let font = FontArc::try_from_vec(font_data).expect("Error parsing font");
        let glyph_brush = GlyphBrushBuilder::using_font(font).build();

        let mut font_map = HashMap::new();
        font_map.insert("default".to_string(), FontId(0));
        // Weiholmir is the default, so map it too
        font_map.insert("weiholmir".to_string(), FontId(0));

        Self {
            glyph_brush,
            font_map,
        }
    }

    pub fn get_font_id(&self, family_name: Option<&str>) -> FontId {
        match family_name {
            Some(name) => *self.font_map.get(name).unwrap_or(&FontId(0)),
            None => FontId(0),
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
        let section = Section {
            text: vec![Text::new(text)
                .with_scale(PxScale::from(font_size))
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
