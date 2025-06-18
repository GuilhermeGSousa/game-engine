use ecs::resource::Resource;

#[derive(Resource)]
pub struct PhysicsServer {}

impl PhysicsServer {
    pub fn new() -> Self {
        Self {}
    }
}
