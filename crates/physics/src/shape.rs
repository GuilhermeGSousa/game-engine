use std::{collections::HashMap, sync::Arc};

use ecs::{
    events::event_reader::EventReader, CommandQueue, Component, Entity, Query, Res, ResMut,
    Resource, With,
};
use essential::assets::{asset_store::AssetStore, handle::AssetLifetimeEvent, AssetId};
use log::warn;
use mesh::{Mesh, MeshComponent};

use crate::{
    backend::{MeshShapeCreationError, PhysicsBackend},
    collider::Collider,
    ActiveBackend,
};

pub struct PhysicsShape(pub(crate) <ActiveBackend as PhysicsBackend>::ShapeHandle);

impl PhysicsShape {
    pub fn from_mesh(mesh: &Mesh) -> Result<Self, MeshShapeCreationError> {
        Ok(Self(ActiveBackend::create_shape_from_mesh(mesh)?))
    }

    pub fn create_sphere_shape(radius: f32) -> Self {
        Self(ActiveBackend::create_sphere_shape(radius))
    }

    pub fn create_cuboid_shape(width: f32, height: f32, length: f32) -> Self {
        Self(ActiveBackend::create_cuboid_shape(width, height, length))
    }

    pub fn create_capsule_shape(half_height: f32, radius: f32) -> Self {
        Self(ActiveBackend::create_capsule_shape(half_height, radius))
    }
}

pub type SharedPhysicsShape = Arc<PhysicsShape>;

#[derive(Resource, Default)]
pub struct PhysicsMeshShapes {
    cache: HashMap<AssetId, SharedPhysicsShape>,
}

impl PhysicsMeshShapes {
    pub(crate) fn get_or_create_mesh_shape(
        &mut self,
        asset_id: AssetId,
        mesh: &Mesh,
    ) -> Result<SharedPhysicsShape, MeshShapeCreationError> {
        if let Some(shape) = self.cache.get(&asset_id) {
            return Ok(shape.clone());
        }

        let shape = SharedPhysicsShape::new(PhysicsShape::from_mesh(mesh)?);
        self.cache.insert(asset_id, shape.clone());
        Ok(shape)
    }

    pub(crate) fn drop_shape(&mut self, asset_id: &AssetId) {
        self.cache.remove(&asset_id);
    }
}

#[derive(Component)]
pub struct PhysicsMeshShapeGenerator;

pub(crate) fn generate_mesh_shapes(
    meshes_to_generate: Query<(Entity, &MeshComponent), With<PhysicsMeshShapeGenerator>>,
    meshes: Res<AssetStore<Mesh>>,
    mut mesh_shapes: ResMut<PhysicsMeshShapes>,
    mut cmd: CommandQueue,
) {
    for (entity, mesh_comp) in meshes_to_generate.iter() {
        let Some(mesh) = meshes.get(&mesh_comp.handle) else {
            continue;
        };

        cmd.remove::<PhysicsMeshShapeGenerator>(entity);

        // The mesh data will not change, so a rejected mesh would be rejected
        // identically every frame: drop the request rather than retrying.
        let shape = match mesh_shapes.get_or_create_mesh_shape(mesh_comp.handle.id(), mesh) {
            Ok(shape) => shape,
            Err(error) => {
                warn!("Skipping collider for entity {entity:?}: {error}");
                continue;
            }
        };

        cmd.insert(Collider::from_shape(shape), entity);
    }
}

pub(crate) fn clean_shapes_for_dropped_meshes(
    mut mesh_shapes: ResMut<PhysicsMeshShapes>,
    dropped_events: EventReader<AssetLifetimeEvent>,
) {
    for e in dropped_events.read() {
        match e {
            AssetLifetimeEvent::Dropped(asset_id, _) => {
                mesh_shapes.drop_shape(asset_id);
            }
        }
    }
}
