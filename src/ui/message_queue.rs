//! Central message queue for UI -> Game communication.
//!
//! This queue handles global messages that need to be processed by the game loop.
//! View-specific actions are handled directly within their views via context refs.
//!
#[derive(PartialEq)]
pub enum UiMessage {
    // LeftMouseClicked,
    WindowShouldClose,
    ReloadWorldData,
    RenderStagedEmitters { do_it: bool },
    ApplySettings,
    CancelSettings,
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
