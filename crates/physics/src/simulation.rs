use ecs::{query::Query, resource::ResMut};
use essential::transform::Transform;

use crate::{
    body::BodyId, interpolation::TransformInterpolation, physics_pipeline::PhysicsPipeline,
    physics_state::PhysicsState,
};

pub fn step_simulation(
    query: Query<(&BodyId, &mut Transform, Option<&mut TransformInterpolation>)>,
    mut pipeline: ResMut<PhysicsPipeline>,
    mut state: ResMut<PhysicsState>,
) {
    {
        profiling::scope!("jolt::step");
        pipeline.step(&mut state);
    }

    {
        profiling::scope!("jolt::write_back_transforms");
        for (body_id, mut transform, interpolation) in query.iter() {
            let stepped_transform = state.body_transform(*body_id);
            if let Some(mut interpolation) = interpolation {
                interpolation.push(&stepped_transform);
            }
            **transform = stepped_transform;
        }
    }
}
