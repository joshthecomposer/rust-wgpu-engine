use glfw::Glfw;
use glfw;

pub struct Time {
    // variable/render timing
    pub now: f32,
    pub last: f32,
    pub dt: f32,
    pub elapsed: f32,

    // Fixed step timings
    pub fixed_dt: f32,
    pub accumulator: f32,
    pub max_frame_dt: f32,

    // interpolation
    pub alpha: f32, // accumulator / fixed_dt (computed after fixed loop)
    pub did_step: bool,
}

impl Time {
    pub fn new(fixed_hz: f32, glfw_time: f32) -> Self {
        Self {
            now: glfw_time,
            last: glfw_time,
            dt: 0.0,
            elapsed: 0.0,
            fixed_dt: 1.0 / fixed_hz,
            accumulator: 0.0,
            max_frame_dt: 0.25,
            alpha: 0.0,
            did_step: false,
        }
    }

    pub fn begin_frame(&mut self, glfw_time: f32) {
        self.now = glfw_time;
        let mut frame_dt = self.now - self.last;
        if frame_dt > self.max_frame_dt { frame_dt = self.max_frame_dt; }
        self.last = self.now;

        self.dt = frame_dt;
        self.elapsed += frame_dt;

        self.accumulator += frame_dt;

        self.did_step = false;
    }

    pub fn should_step(&self) -> bool {
        self.accumulator >= self.fixed_dt
    }

    pub fn begin_fixed_step(&mut self) {
        if !self.did_step { self.did_step = true; }
    }

    pub fn end_fixed_step(&mut self) {
        self.accumulator -= self.fixed_dt;
    }

    pub fn end_frame(&mut self) {
        self.alpha = (self.accumulator / self.fixed_dt).clamp(0.0, 1.0);
    }


}

