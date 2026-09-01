use ecs::{
    command::CommandQueue, component::Component, entity::Entity, query::Query, resource::Res,
};
use essential::assets::{asset_store::AssetStore, handle::AssetHandle};
use essential::transform::Transform;
use mesh::mesh::MeshComponent;
use render::components::material::MaterialComponent;
use render::components::render_entity::SyncWithRenderWorld;

use crate::scene::Scene;

/// Attach to an entity to have [`spawn_scene_components`] expand the referenced
/// [`Scene`] into a child entity hierarchy on the next run.
#[derive(Component)]
pub struct SceneSpawnerComponent(pub AssetHandle<Scene>);

/// Expands every entity carrying a [`SceneSpawnerComponent`] whose scene asset
/// has finished loading into one entity per `SceneNode`, wiring up
/// mesh/material components and the parent/child hierarchy. Root nodes are
/// parented to the spawner entity so they inherit its transform.
pub fn spawn_scene_components(
    mut cmd: CommandQueue,
    spawners: Query<(Entity, &SceneSpawnerComponent, Option<&Transform>)>,
    scenes: Res<AssetStore<Scene>>,
) {
    for (spawner_entity, spawner, spawner_transform) in spawners.iter() {
        let Some(scene) = scenes.get(&spawner.0) else {
            continue;
        };

        if spawner_transform.is_none() {
            cmd.insert(Transform::IDENTITY, spawner_entity);
        }

        let mut node_entities = Vec::with_capacity(scene.nodes.len());
        for node in &scene.nodes {
            node_entities.push(cmd.spawn(node.transform.clone()).entity());
        }

        for (index, node) in scene.nodes.iter().enumerate() {
            let Some(mesh) = &node.mesh else {
                continue;
            };
            cmd.insert(
                (
                    MeshComponent {
                        handle: mesh.clone(),
                    },
                    SyncWithRenderWorld,
                ),
                node_entities[index],
            );
            if let Some(material) = &node.material {
                cmd.insert(
                    MaterialComponent {
                        handle: material.clone(),
                    },
                    node_entities[index],
                );
            }
        }

        let mut has_parent = vec![false; scene.nodes.len()];
        for (index, node) in scene.nodes.iter().enumerate() {
            for &child in &node.children {
                // A malformed cooked Scene can carry an out-of-range child
                // index; skip it rather than panicking on the main schedule.
                let Some(&child_entity) = node_entities.get(child) else {
                    continue;
                };
                cmd.add_child(node_entities[index], child_entity);
                has_parent[child] = true;
            }
        }
        for (index, node_entity) in node_entities.iter().enumerate() {
            if !has_parent[index] {
                cmd.add_child(spawner_entity, *node_entity);
            }
        }

        cmd.remove::<SceneSpawnerComponent>(spawner_entity);
    }
}
