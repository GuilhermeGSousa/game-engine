use ecs::resource::Resource;

#[derive(Resource)]
pub struct WinitEvents {
    pub winit_events: Vec<winit::event::WindowEvent>,
}

impl WinitEvents {
    pub fn new() -> Self {
        Self {
            winit_events: Vec::new(),
        }
    }
}
