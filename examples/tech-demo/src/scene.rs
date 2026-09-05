use game_engine::{
    ecs::{CommandQueue, Res},
    essential::{assets::asset_server::AssetServer, transform::Transform},
    scene::{scene::Scene, spawner::SceneSpawnerComponent},
};

const FOREST_SCENE: &str = "content/forest/scene.gasset";

pub(crate) fn spawn_scene(mut cmd: CommandQueue, asset_server: Res<AssetServer>) {
    // TODO(asset-import-pipeline): the forest lost its per-scene shadow opt-in
    // (old GLTFSpawnerComponent::with_shadows) — the importer hard-codes
    // Light::shadowmaps_enabled to false.
    cmd.spawn((
        SceneSpawnerComponent(asset_server.load::<Scene>(FOREST_SCENE)),
        Transform::default(),
    ));
}
