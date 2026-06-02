use ecs::{Added, CommandQueue, Component, Query, Res};
use essential::{assets::asset_server::AssetServer, transform::Transform};
use glam::{Quat, Vec3};
use render::{MaterialComponent, components::mesh_component::MeshComponent};
use wgpu::Color;

use crate::{material::DebugGizmoMaterial, shapes::GizmoShapes};

#[derive(Component)]
pub struct GizmoLine {
    pub start: Vec3,
    pub end: Vec3,
    pub color: Color,
}

#[derive(Component)]
pub struct GizmoSphere {
    pub center: Vec3,
    pub radius: f32,
    pub color: Color,
}

#[derive(Component)]
pub struct GizmoCube {}

pub(crate) fn line_added(
    shapes: Res<GizmoShapes>,
    query: Query<&GizmoLine, Added<GizmoLine>>,
    cmd: CommandQueue,
) {
    for line in query.iter() {}
}

pub(crate) fn sphere_added(
    shapes: Res<GizmoShapes>,
    query: Query<&GizmoSphere, Added<GizmoLine>>,
    cmd: CommandQueue,
    asset_server: Res<AssetServer>,
) {
    for sphere in query.iter() {
        let material = MaterialComponent {
            handle: asset_server.add(DebugGizmoMaterial { color: todo!() }),
        };

        cmd.spawn((
            MeshComponent {
                handle: shapes.sphere.clone(),
            },
            material,
            Transform::from_translation_rotation(sphere.center, Quat::IDENTITY),
        ));
    }
}
