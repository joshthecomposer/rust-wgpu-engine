use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub enum Alignment {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct GridSpan(pub u8);

impl Default for GridSpan {
    fn default() -> Self {
        GridSpan(12)
    }
}

impl GridSpan {
    pub fn new(span: u8) -> Self {
        GridSpan(span.clamp(1, 12))
    }

    pub fn fraction(&self) -> f32 {
        self.0 as f32 / 12.0
    }
}




