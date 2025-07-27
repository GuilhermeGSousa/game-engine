use app::plugins::Plugin;

use crate::{
    physics_pipeline::PhysicsPipeline, physics_state::PhysicsState, simulation::step_simulation,
};

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut app::App) {
        app.insert_resource(PhysicsPipeline::new())
            .insert_resource(PhysicsState::new())
            .add_system(
                app::update_group::UpdateGroup::LateFixedUpdate,
                step_simulation,
            );
    }
}
