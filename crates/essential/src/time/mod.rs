use std::time::Duration;

use ecs::resource::Resource;

mod instant;

use crate::time::instant::Instant;

#[derive(Resource)]
pub struct Time {
    last_update: Instant,
    delta: Duration,
}

impl Time {
    const FIXED_DELTA_TIME: f32 = 1.0 / 30.0; // 30 FPS

    pub fn new() -> Self {
        Self {
            last_update: Instant::now(),
            delta: Duration::default(),
        }
    }

    pub fn delta(&self) -> Duration {
        self.delta
    }

    pub fn update(&mut self) {
        self.delta = Instant::now() - self.last_update;
        self.last_update = Instant::now();
    }

    pub fn fixed_delta_time() -> f32 {
        Self::FIXED_DELTA_TIME
    }
}
