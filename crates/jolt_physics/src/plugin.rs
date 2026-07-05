use app::plugins::Plugin;
use ecs::system::schedule::UpdateGroup;

use crate::{
    physics_pipeline::PhysicsPipeline, physics_state::PhysicsState, rigid_body::RigidBody,
    simulation::step_simulation,
};

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_component_lifecycle::<RigidBody>();
        app.insert_resource(PhysicsPipeline::new())
            .insert_resource(PhysicsState::new())
            .add_system(UpdateGroup::LateFixedUpdate, step_simulation);
    }
}
