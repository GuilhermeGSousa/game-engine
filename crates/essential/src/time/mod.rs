use std::time::Duration;

use ecs::resource::Resource;

mod frame_stats;
mod instant;

pub use frame_stats::FrameStats;

use crate::time::instant::Instant;

#[derive(Resource)]
pub struct Time {
    last_update: Instant,
    delta: Duration,
    fixed_overstep: f32,
}

impl Time {
    const FIXED_DELTA_TIME: f32 = 1.0 / 30.0; // 30 FPS

    pub fn new() -> Self {
        Self {
            last_update: Instant::now(),
            delta: Duration::default(),
            fixed_overstep: 0.0,
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

    /// Seconds elapsed since the last fixed step. The physics world is frozen between
    /// steps, so systems running at frame rate must extrapolate by this to see current state.
    pub fn fixed_overstep(&self) -> f32 {
        self.fixed_overstep
    }

    pub fn set_fixed_overstep(&mut self, fixed_overstep: f32) {
        self.fixed_overstep = fixed_overstep;
    }

    /// Fraction of the way from the last fixed step to the next, for interpolating
    /// fixed-step state into frame-rate rendering.
    pub fn fixed_alpha(&self) -> f32 {
        (self.fixed_overstep / Self::FIXED_DELTA_TIME).clamp(0.0, 1.0)
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}
