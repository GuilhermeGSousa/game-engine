use ecs::{query::Query, resource::ResMut};
use essential::transform::Transform;

use crate::{
    physics_pipeline::PhysicsPipeline, physics_state::PhysicsState, rigid_body::RigidBody,
};

pub fn step_simulation(
    query: Query<(&RigidBody, &mut Transform)>,
    mut pipeline: ResMut<PhysicsPipeline>,
    mut state: ResMut<PhysicsState>,
) {
    pipeline.step(&mut state);

    for (rigid_body, transform) in query.iter() {
        *transform = state.get_rigid_body(rigid_body);
    }
}
