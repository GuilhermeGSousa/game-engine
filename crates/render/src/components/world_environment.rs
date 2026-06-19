use color::LinearRgba;
use ecs::resource::Resource;
use encase::ShaderType;

// TODO: Actually use this
#[derive(Resource, ShaderType)]
pub struct WorldEnvironment {
    ambient_color: LinearRgba,
}

impl WorldEnvironment {
    pub fn new(ambient_color: LinearRgba) -> Self {
        Self { ambient_color }
    }

    pub fn ambient_color(&self) -> &LinearRgba {
        &self.ambient_color
    }
}
