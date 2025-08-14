use ecs::resource::Resource;
use encase::ShaderType;
use glam::Vec4;

// TODO: Actually use this
#[derive(Resource, ShaderType)]
pub struct WorldEnvironment {
    ambient_color: Vec4,
}

impl WorldEnvironment {
    pub fn new(ambient_color: Vec4) -> Self {
        Self { ambient_color }
    }

    pub fn ambient_color(&self) -> &Vec4 {
        &self.ambient_color
    }
}
