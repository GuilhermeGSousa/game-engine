use ecs::{
    entity::{
        hierarchy::{ChildOf, Children},
        Entity,
    },
    query::{query_filter::Without, Query},
};

use crate::transform::{GlobalTranform, Transform};

pub fn propagate_global_transforms(
    roots: Query<(Entity, &Children, &GlobalTranform), Without<ChildOf>>,
    children: Query<(Entity, &mut GlobalTranform, &Transform)>,
) {
    for (root_entity, root_children, root_transform) in roots.iter() {
        propagate_to_children(root_entity, root_transform, &children);
    }
}

fn propagate_to_children(
    parent_entity: Entity,
    parent_transform: &GlobalTranform,
    children: &Query<(Entity, &mut GlobalTranform, &Transform)>,
) {
}
