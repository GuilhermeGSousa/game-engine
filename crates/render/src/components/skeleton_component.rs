use crate::assets::skeleton::Skeleton;
use ecs::{component::Component, entity::Entity};
use essential::assets::handle::AssetHandle;

#[derive(Component)]
pub struct SkeletonComponent {
    skeleton: AssetHandle<Skeleton>,
    joints: Vec<Entity>,
}

pub(crate) fn skeleton_added()
{}

pub(crate) fn skeleton_changed()
{}