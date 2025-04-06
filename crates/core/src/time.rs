use ecs::resource::Resource;
use std::time::Instant;

#[derive(Resource)]
pub struct Time {
    last_update: Instant,
}

impl Time {
    pub fn new() -> Self {
        Self {
            last_update: Instant::now(),
        }
    }

    pub fn delta(&self) -> f32 {
        self.last_update.elapsed().as_secs_f32()
    }

    pub fn update(&mut self) {
        self.last_update = Instant::now();
    }
}
