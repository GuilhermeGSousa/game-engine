use app::plugins::Plugin;
use ecs::system::schedule::UpdateGroup;

use crate::{
    collider::Collider, ground::probe_ground, physics_pipeline::PhysicsPipeline,
    physics_state::PhysicsState, simulation::step_simulation,
};

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_component_lifecycle::<Collider>();
        app.insert_resource(PhysicsPipeline::new())
            .insert_resource(PhysicsState::new())
            // probe_ground must be registered after step_simulation: their
            // conflicting PhysicsState access gives an insertion-order edge.
            .add_system(UpdateGroup::LateFixedUpdate, step_simulation)
            .add_system(UpdateGroup::LateFixedUpdate, probe_ground);
    }
}
