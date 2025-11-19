use std::collections::{vec_deque::Drain, VecDeque};

#[derive(PartialEq)]
pub enum UiMessage {
    LeftMouseClicked,
    WindowShouldClose,
    PauseToggle,
    ReloadWorldData,
}

pub struct MessageQueue {
    pub queue: VecDeque<UiMessage>,
}

impl MessageQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn send(&mut self, msg: UiMessage) {
        self.queue.push_back(msg);
    }

    pub fn drain(&mut self) -> Vec<UiMessage> {
        self.queue.drain(..).into_iter().collect()
    }
}
