use ecs::resource::Resource;

const FIXED_DELTA_TIME: f32 = 1.0 / 30.0; // 30 FPS

#[derive(Resource)]
pub struct Time {
    last_update: web_time::Instant,
}

// TODO: Fix this for WASM
impl Time {
    pub fn new() -> Self {
        Self {
            last_update: web_time::Instant::now(),
        }
    }

    pub fn delta(&self) -> f32 {
        self.last_update.elapsed().as_secs_f32()
    }

    pub fn update(&mut self) {
        self.last_update = web_time::Instant::now();
    }

    pub fn fixed_delta_time() -> f32 {
        FIXED_DELTA_TIME
    }
}
