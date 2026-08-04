use game_engine::{
    ecs::{CommandQueue, Res},
    essential::{assets::asset_server::AssetServer, transform::Transform},
    gltf_loader::loader::GLTFSpawnerComponent,
};

const FOREST_PATH: &str = "res/forest.glb";

pub(crate) fn spawn_scene(mut cmd: CommandQueue, asset_server: Res<AssetServer>) {
    cmd.spawn((
        GLTFSpawnerComponent::from_handle(asset_server.load(FOREST_PATH)).with_shadows(),
        Transform::default(),
    ));
}
