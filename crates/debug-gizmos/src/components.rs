use color::LinearRgba;
use ecs::{Added, CommandQueue, Component, Entity, Query, Res};
use essential::{assets::asset_server::AssetServer, transform::Transform};
use glam::{Quat, Vec3};
use render::{MaterialComponent, components::mesh_component::MeshComponent};

use crate::{material::DebugGizmoMaterial, shapes::GizmoShapes};

#[derive(Component)]
pub struct GizmoLine {
    pub start: Vec3,
    pub end: Vec3,
    pub color: LinearRgba,
}

#[derive(Component)]
pub struct GizmoSphere {
    pub center: Vec3,
    pub radius: f32,
    pub color: LinearRgba,
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
    query: Query<(Entity, &GizmoSphere), Added<GizmoSphere>>,
    mut cmd: CommandQueue,
    asset_server: Res<AssetServer>,
) {
    for (_entity, sphere) in query.iter() {
        let material = MaterialComponent {
            handle: asset_server.add(DebugGizmoMaterial { color: sphere.color }),
        };

        cmd.spawn((
            MeshComponent {
                handle: shapes.sphere.clone(),
            },
            material,
            Transform::from_translation_rotation_scale(
                sphere.center,
                Quat::IDENTITY,
                Vec3::splat(sphere.radius),
            ),
        ));

        // Maybe despawn entity?
    }
}
