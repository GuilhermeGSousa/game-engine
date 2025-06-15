use ecs::{component::Component, query::Query, query_filter::Added};

use crate::collision::collider_shape::CollisionShape;

#[derive(Component)]
pub struct Collider {
    pub shape: CollisionShape,
}

pub(crate) fn register_colliders(query: Query<(&Collider,), Added<(Collider,)>>) {
    for collider in query.iter() {
        print!("Added Collider");
    }
}
