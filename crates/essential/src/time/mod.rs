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

    /// Upper bound on the delta reported by a single [`Time::update`] call. Without this, a
    /// stalled frame (a debugger pause, a slow/software-rendered frame, the OS descheduling the
    /// process) produces one huge `delta`, which `App::update`'s fixed-timestep accumulator
    /// then has to drain in one go — every `FixedUpdate` system (movement, physics) advances by
    /// that same huge real-world gap, so a character can fly across an entire level in a single
    /// frame. Clamping means a long stall is simulated as several seconds of dropped/slowed time
    /// rather than teleporting the world forward to match the wall clock.
    const MAX_DELTA_TIME: Duration = Duration::from_millis(250);

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
        self.delta = (Instant::now() - self.last_update).min(Self::MAX_DELTA_TIME);
        self.last_update = Instant::now();
    }

    pub fn fixed_delta_time() -> f32 {
        Self::FIXED_DELTA_TIME
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}
