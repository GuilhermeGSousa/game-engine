use ecs::{Added, Component, Query};
use render::MaterialComponent;

use crate::material::DebugGizmoMaterial;



#[derive(Component)]
pub struct GizmoLine
{

}

#[derive(Component)]
pub struct GizmoSphere
{

}

#[derive(Component)]
pub struct GizmoCube
{

}

pub(crate) fn line_added(
    query: Query<&GizmoLine, Added<GizmoLine>>
)
{

}