use ecs::{
    entity::{
        hierarchy::{ChildOf, Children},
        Entity,
    },
    query::{
        query_filter::{Changed, Without},
        Query,
    },
};

use crate::transform::{GlobalTranform, Transform};

pub fn update_simple_entities(
    roots: Query<
        (&Transform, &mut GlobalTranform),
        (Changed<Transform>, Without<ChildOf>, Without<Children>),
    >,
) {
    for (local_transform, mut global_transform) in roots.iter() {
        global_transform.set_matrix(local_transform.compute_matrix());
    }
}

pub fn propagate_global_transforms(
    roots: Query<(&Children, &mut GlobalTranform, &Transform), Without<ChildOf>>,
    transform_query: Query<(Entity, &mut GlobalTranform, &Transform, Option<&Children>)>,
) {
    for (root_children, mut root_global_transform, root_local_transform) in roots.iter() {
        root_global_transform.set_matrix(root_local_transform.compute_matrix());
        propagate_to_children(&root_global_transform, root_children, &transform_query);
    }
}

fn propagate_to_children(
    parent_transform: &GlobalTranform,
    children: &Children,
    transform_query: &Query<(Entity, &mut GlobalTranform, &Transform, Option<&Children>)>,
) {
    for child in children {
        if let Some((_, mut child_global_transform, child_local_transform, grand_children)) =
            transform_query.get_entity(*child)
        {
            child_global_transform
                .set_matrix(parent_transform.matrix() * child_local_transform.compute_matrix());

            if let Some(grand_children) = grand_children {
                propagate_to_children(&child_global_transform, grand_children, transform_query);
            }
        }
    }
}
