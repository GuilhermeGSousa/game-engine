use ecs::{entity::Entity, query::Query, resource::ResMut};
use essential::transform::Transform;

use crate::{
    physics_pipeline::PhysicsPipeline, physics_state::PhysicsState, rigid_body::RigidBody,
};

pub fn step_simulation(
    query: Query<(Entity, &RigidBody, &mut Transform)>,
    mut pipeline: ResMut<PhysicsPipeline>,
    mut state: ResMut<PhysicsState>,
) {
    pipeline.step(&mut state);

    for (entity, _rigid_body, mut transform) in query.iter() {
        let Some(body) = state.get_body(entity) else {
            continue;
        };
        **transform = state.body_transform(body);
    }
}
