use game_engine::animation::graph::AnimationGraph;
use game_engine::animation::node::state_machine::AnimationStateMachine;
use game_engine::animation::player::{AnimationHandleComponent, AnimationPlayer};
use game_engine::ecs::{Entity, Query, With, Without};
use game_engine::essential::time::Time;
use game_engine::essential::transform::Transform;
use game_engine::gltf_loader::loader::GLTFInstance;
use game_engine::jolt_physics::collider::Collider;
use game_engine::jolt_physics::rigid_body::{AllowedDofs, RigidBody};
use game_engine::window::input::{Input, KeyCode, PhysicalKey};
use game_engine::{
    color::LinearRgba,
    ecs::{CommandQueue, Component, Res, Resource},
    essential::assets::{asset_server::AssetServer, asset_store::AssetStore, handle::AssetHandle},
    gameplay::player::spawn_first_person_player,
    gltf_loader::loader::{GLTFScene, GLTFSpawnerComponent, GLTFUsageSettings},
    render::components::{Light, light::LightType::Point},
};
use glam::{Quat, Vec2, Vec3};

const CHAR_ASSET: &str = "res/UAL1.glb";

#[derive(Component)]
pub(crate) struct Player;

#[derive(Component)]
pub(crate) struct AnimationsReady;

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
        Vec3::Y * 2.0,
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
        RigidBody {
            density: 1000.0,
            allowed_dofs: AllowedDofs::TRANSLATION | AllowedDofs::ROTATION_Y,
            ..Default::default()
        },
        Collider::Capsule {
            half_height: 1.0,
            radius: 1.0,
        },
        Transform::from_translation_rotation(Vec3::new(0.0, 10.0, -5.0), Quat::IDENTITY),
    ));
}

pub(crate) fn setup_character_animations(
    char_asset: Res<GLTFCharacterAsset>,
    players: Query<(Entity, &GLTFInstance), (With<Player>, Without<AnimationsReady>)>,
    server: Res<AssetServer>,
    mut cmd: CommandQueue,
    gltf_store: Res<AssetStore<GLTFScene>>,
) {
    let Some(gltf_char) = gltf_store.get(&char_asset.0) else {
        return;
    };

    // The GLTFInstance only exists once the scene has finished spawning; follow it
    // straight to the animated node instead of scanning every AnimationPlayer.
    let Some((player_entity, instance)) = players.iter().next() else {
        return;
    };

    let Some(entity) = instance.animation_player() else {
        return;
    };

    let (
        Some(idle),
        Some(jog),
        Some(jog_fw_l),
        Some(jog_fw_r),
        Some(jog_l),
        Some(jog_r),
        Some(jog_bw),
        Some(job_bw_l),
        Some(job_bw_r),
    ) = (
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
        gltf_char
            .get_animation("Jog_Bwd_Loop")
            .map(|anim| anim.handle()),
        gltf_char
            .get_animation("Jog_Bwd_L_Loop")
            .map(|anim| anim.handle()),
        gltf_char
            .get_animation("Jog_Bwd_R_Loop")
            .map(|anim| anim.handle()),
    )
    else {
        return;
    };

    let mut movement_graph = AnimationGraph::new();
    movement_graph.result_node().with_blend_space_2d_input(
        |blackboard| blackboard.get_vec2("movement").unwrap_or(Vec2::ZERO),
        |context| {
            context
                .animation_clip_input(idle, Vec2::ZERO)
                .animation_clip_input(jog, Vec2::new(0.0, 1.0))
                .animation_clip_input(jog_fw_l, Vec2::new(-1.0, 1.0))
                .animation_clip_input(jog_fw_r, Vec2::new(1.0, 1.0))
                .animation_clip_input(jog_l, Vec2::new(-1.0, 0.0))
                .animation_clip_input(jog_r, Vec2::new(1.0, 0.0))
                .animation_clip_input(jog_bw, Vec2::new(0.0, -1.0))
                .animation_clip_input(job_bw_l, Vec2::new(-1.0, -1.0))
                .animation_clip_input(job_bw_r, Vec2::new(1.0, -1.0));
        },
    );

    let mut graph = AnimationGraph::new();
    graph.result_node().with_input(
        AnimationStateMachine::from_initial_state(
            "movement",
            server.add(movement_graph),
            |_transition| {},
        )
        .build(),
        |_node_context| {},
    );

    cmd.insert(AnimationHandleComponent::new(server.add(graph)), entity);
    cmd.insert(AnimationsReady, player_entity);
}

pub(crate) fn update_movement(
    anim_players: Query<&mut AnimationPlayer>,
    input: Res<Input>,
    time: Res<Time>,
) {
    for mut anim_player in anim_players.iter() {
        let mut input_vec = anim_player.get_vec2_param("movement").unwrap_or(Vec2::ZERO);

        let mut added_input = Vec2::ZERO;
        if input.is_held(PhysicalKey::Code(KeyCode::ArrowUp)) {
            added_input += Vec2::Y;
        }

        if input.is_held(PhysicalKey::Code(KeyCode::ArrowDown)) {
            added_input -= Vec2::Y;
        }

        if input.is_held(PhysicalKey::Code(KeyCode::ArrowRight)) {
            added_input += Vec2::X;
        }

        if input.is_held(PhysicalKey::Code(KeyCode::ArrowLeft)) {
            added_input -= Vec2::X;
        }

        let dt = time.delta().as_secs_f32();
        input_vec += added_input * 5.0 * dt;

        if added_input.x == 0.0 {
            input_vec.x *= (1.0 - 5.0 * dt).max(0.0);
        }
        if added_input.y == 0.0 {
            input_vec.y *= (1.0 - 5.0 * dt).max(0.0);
        }

        input_vec = input_vec.clamp(Vec2::NEG_ONE, Vec2::ONE);
        anim_player.set_vec2_param("movement", input_vec);
    }
}
