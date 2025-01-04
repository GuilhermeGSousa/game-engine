use core::world::World;

pub struct App {
    world: World,
}

impl App {
    pub fn new() -> App {
        Self {
            world: World::new(),
        }
    }

    pub fn run(&mut self) {}
}
