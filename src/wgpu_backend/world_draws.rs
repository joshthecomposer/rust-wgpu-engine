use crate::enums_types::InstanceUniform;

/// Per-frame scratch and batch storage reused by the renderer.
pub struct WorldDraws {
    pub static_scratch: Vec<InstanceUniform>,
}

impl WorldDraws {
    pub fn new() -> Self {
        Self {
            static_scratch: Vec::new(),
        }
    }
}
