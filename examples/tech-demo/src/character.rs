use game_engine::animation::graph::AnimationGraph;
use game_engine::essential::transform::Transform;
use game_engine::{
    color::LinearRgba,
    ecs::{CommandQueue, Component, Res, Resource},
    essential::assets::{asset_server::AssetServer, asset_store::AssetStore, handle::AssetHandle},
    gameplay::player::spawn_first_person_player,
    gltf_loader::loader::{GLTFScene, GLTFSpawnerComponent, GLTFUsageSettings},
    render::components::{Light, light::LightType::Point},
    world_grid::WorldGrid,
};
use glam::{Quat, Vec2, Vec3};

const CHAR_ASSET: &str = "res/UAL1.glb";

#[derive(Component)]
pub(crate) struct Player;

#[derive(Resource)]
pub(crate) struct GLTFCharacterAsset(AssetHandle<GLTFScene>);

pub(crate) fn spawn_character(asset_server: Res<AssetServer>, mut cmd: CommandQueue) {
    let char_handle = asset_server.load_with_usage_settings::<GLTFScene>(
        CHAR_ASSET,
        GLTFUsageSettings {
            root_bone: Some("root"),
        },
    );

    spawn_first_person_player(
        &mut cmd,
        Vec3::ZERO,
        Light {
            color: LinearRgba::WHITE,
            intensity: 10.0,
            light_type: Point,
        },
    );

    cmd.insert_resource(GLTFCharacterAsset(char_handle.clone()));

    cmd.spawn((
        Player,
        GLTFSpawnerComponent(char_handle),
        Transform::from_translation_rotation(Vec3::new(0.0, -1.0, -5.0), Quat::IDENTITY),
    ));
    cmd.spawn(WorldGrid::default());
}

pub(crate) fn setup_character_animations(
    char_asset: Res<GLTFCharacterAsset>,
    cmd: CommandQueue,
    gltf_store: AssetStore<GLTFScene>,
) {
    let Some(gltf_char) = gltf_store.get(&char_asset.0) else {
        return;
    };

    let (Some(idle), Some(jog), Some(jog_fw_l), Some(jog_fw_r), Some(jog_l), Some(jog_r)) = (
        gltf_char
            .get_animation("Idle_Loop")
            .map(|anim| anim.handle()),
        gltf_char
            .get_animation("Jog_Fwd_Loop")
            .map(|anim| anim.handle()),
        gltf_char
            .get_animation("Jog_Fwd_L_Loop")
            .map(|anim| anim.handle()),
        gltf_char
            .get_animation("Jog_Fwd_R_Loop")
            .map(|anim| anim.handle()),
        gltf_char
            .get_animation("Jog_Left_Loop")
            .map(|anim| anim.handle()),
        gltf_char
            .get_animation("Jog_Right_Loop")
            .map(|anim| anim.handle()),
    ) else {
        return;
    };

    AnimationGraph::new()
        .result_node()
        .with_blend_space_2d_input(
            |blackboard| blackboard.get_vec2("movement").unwrap_or(Vec2::ZERO),
            |context| {
                context
                    .animation_clip_input(idle, Vec2::ZERO)
                    .animation_clip_input(jog, Vec2::new(0.0, 1.0))
                    .animation_clip_input(jog_fw_l, Vec2::new(1.0, 1.0))
                    .animation_clip_input(jog_fw_r, Vec2::new(-1.0, 1.0))
                    .animation_clip_input(jog_l, Vec2::new(-1.0, 0.0))
                    .animation_clip_input(jog_r, Vec2::new(0.0, 1.0));
            },
        );
}
