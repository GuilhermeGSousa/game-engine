use ecs::{query::Query, resource::ResMut};
use essential::transform::Transform;

use crate::{body::BodyId, physics_pipeline::PhysicsPipeline, physics_state::PhysicsState};

pub fn step_simulation(
    query: Query<(&BodyId, &mut Transform)>,
    mut pipeline: ResMut<PhysicsPipeline>,
    mut state: ResMut<PhysicsState>,
) {
    {
        profiling::scope!("jolt::step");
        pipeline.step(&mut state);
    }

    {
        profiling::scope!("jolt::write_back_transforms");
        for (body_id, mut transform) in query.iter() {
            **transform = state.body_transform(*body_id);
        }
    }
}
