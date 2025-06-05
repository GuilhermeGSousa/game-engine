use ecs::{entity::Entity, query::Query, world::World};
use essential::transform::Transform;

use crate::collision::collider::Collider;

pub(crate) struct ContactInfo {}

fn broad_phase(colliders: &Query<(&Collider, &Transform)>) -> Vec<(Entity, Entity)> {
    // TODO: Implement broad phase collision detection
    Vec::new()
}

fn narrow_phase(
    collider_a: &Collider,
    transform_a: &Transform,
    collider_b: &Collider,
    transform_b: &Transform,
) -> Option<ContactInfo> {
    None
}

pub(crate) fn resolve_contacts(world: &mut World) {
    let query = Query::<(&Collider, &Transform)>::new(world.as_unsafe_world_cell_ref());

    let pairs = broad_phase(&query);

    for (entity_a, entity_b) in pairs {
        let (collider_a, transform_a) = query.get_entity(entity_a).unwrap();
        let (collider_b, transform_b) = query.get_entity(entity_b).unwrap();

        if let Some(_) = narrow_phase(collider_a, transform_a, collider_b, transform_b) {
            // Handle the contact resolution logic here
            // This could involve updating the physics bodies, applying forces, etc.
            // For now, we will just print the contact information
            println!("Contact detected");
        }
    }
}
