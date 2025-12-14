use std::collections::{vec_deque::Drain, VecDeque};

use glam::Vec3;

use crate::config::emitter_data::EmitterBlackboard;

#[derive(PartialEq)]
pub enum UiMessage {
    LeftMouseClicked,
    WindowShouldClose,
    PauseToggle,
    ReloadWorldData,
    RenderStagedEmitters { do_it: bool },
}

pub struct MessageQueue {
    pub queue: Vec<UiMessage>,
}

impl MessageQueue {
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }

    pub fn send(&mut self, msg: UiMessage) {
        self.queue.push(msg);
    }

    pub fn drain(&mut self) -> Vec<UiMessage> {
        self.queue.drain(..).into_iter().collect()
    }
}
