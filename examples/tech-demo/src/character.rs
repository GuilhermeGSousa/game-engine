use std::f32::consts::PI;

use game_engine::animation::graph::AnimationGraph;
use game_engine::animation::node::AnimationClipNode;
use game_engine::animation::node::AnimationPlayMode::PlayOnce;
use game_engine::animation::node::state_machine::{AnimationFSMTrigger, AnimationStateMachine};
use game_engine::animation::player::{AnimationHandleComponent, AnimationPlayer};
use game_engine::ecs::{Entity, Query, ResMut, With, Without};
use game_engine::essential::transform::Transform;
use game_engine::gameplay::camera::{CameraPivot, EntityFollow};
use game_engine::gltf_loader::loader::GLTFInstance;
use game_engine::physics::body::BodyId;
use game_engine::physics::collider::{Collider, ColliderOffset};
use game_engine::physics::ground::GroundProbe;
use game_engine::physics::movement::CharacterMovement;
use game_engine::physics::physics_state::PhysicsState;
use game_engine::physics::rigid_body::MotionType::Dynamic;
use game_engine::physics::rigid_body::{AllowedDofs, RigidBody};
use game_engine::render::components::Camera;
use game_engine::window::input::{Input, KeyCode, PhysicalKey};
use game_engine::{
    ecs::{CommandQueue, Component, Res, Resource},
    essential::assets::{asset_server::AssetServer, asset_store::AssetStore, handle::AssetHandle},
    gltf_loader::loader::{GLTFScene, GLTFSpawnerComponent, GLTFUsageSettings},
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

    cmd.insert_resource(GLTFCharacterAsset(char_handle.clone()));

    let collider = Collider::capsule(2.0, 1.0);
    let offset = ColliderOffset::bottom_origin(&collider);
    let character = cmd
        .spawn((
            Player,
            GLTFSpawnerComponent(char_handle),
            RigidBody {
                density: 1000.0,
                allowed_dofs: AllowedDofs::TRANSLATION | AllowedDofs::ROTATION_Y,
                motion_type: Dynamic,
            },
            CharacterMovement::new(0.1),
            collider,
            // Origin at the capsule's bottom so the GLTF skeleton root sits on
            // the ground instead of floating at the capsule center.
            offset,
            Transform::from_translation_rotation(Vec3::new(0.0, 10.0, -5.0), Quat::IDENTITY),
            GroundProbe::default(),
        ))
        .entity();
    //

    cmd.spawn((
        CameraPivot::default(),
        Transform::default(),
        EntityFollow {
            target: character,
            offset: Vec3::Y,
        },
    ))
    .add_child((
        Camera::default(),
        Transform::from_translation_rotation(Vec3::NEG_Z * 10.0, Quat::from_rotation_y(PI)),
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
        Some(_jump_start),
        Some(jump_loop),
        Some(jump_land),
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
        gltf_char
            .get_animation("Jump_Start")
            .map(|anim| anim.handle()),
        gltf_char
            .get_animation("Jump_Loop")
            .map(|anim| anim.handle()),
        gltf_char
            .get_animation("Jump_Land")
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
    let mut fsm_node_index = None;
    graph.result_node().with_input(
        AnimationStateMachine::from_initial_state(
            "movement",
            server.add(movement_graph),
            |transition| {
                transition
                    // TODO: not supported yet, jump need to be triggered by an animation event
                    // .to(
                    //     "jump_start",
                    //     AnimationFSMTrigger::on_bool("jumped", true),
                    //     0.01,
                    // );
                    .to(
                        "air",
                        AnimationFSMTrigger::on_bool("is_grounded", false),
                        0.1,
                    );
            },
        )
        // .state(
        //     "jump_start",
        //     server.add(AnimationGraph::from_node(
        //         AnimationClipNode::new(jump_start).with_play_mode(PlayOnce),
        //     )),
        //     |transition| {
        //         transition.to("air", AnimationFSMTrigger::OnAnimationEnd, 0.1);
        //     },
        // )
        .state(
            "air",
            server.add(AnimationGraph::from_node(AnimationClipNode::new(jump_loop))),
            |transition| {
                transition
                    .to(
                        "land",
                        AnimationFSMTrigger::on_bool("is_grounded", true),
                        0.1,
                    )
                    .to(
                        "movement",
                        AnimationFSMTrigger::on_bool("is_grounded", true),
                        0.1,
                    );
            },
        )
        .state(
            "land",
            server.add(AnimationGraph::from_node(
                AnimationClipNode::new(jump_land)
                    .with_play_mode(PlayOnce)
                    .with_start_time(0.1),
            )),
            |transition| {
                transition
                    .to("movement", AnimationFSMTrigger::OnAnimationEnd, 0.1)
                    .to(
                        "jump_start",
                        AnimationFSMTrigger::on_bool("jumped", true),
                        0.1,
                    )
                    .to(
                        "movement",
                        AnimationFSMTrigger::on_non_zero_vec("movement"),
                        0.1,
                    );
            },
        )
        .build(),
        |node_context| {
            fsm_node_index = Some(node_context.index());
        },
    );

    cmd.insert(AnimationHandleComponent::new(server.add(graph)), entity);
    cmd.insert(AnimationsReady, player_entity);
}

pub(crate) fn update_movement(
    movement: Query<(&mut CharacterMovement, &GLTFInstance, &GroundProbe, &BodyId), With<Player>>,
    anim_players: Query<&mut AnimationPlayer>,
    input: Res<Input>,
    mut physics: ResMut<PhysicsState>,
) {
    const PLAYER_SPEED: f32 = 5.0;
    let mut player_input = Vec2::ZERO;
    if input.is_held(PhysicalKey::Code(KeyCode::ArrowUp)) {
        player_input += Vec2::Y;
    }

    if input.is_held(PhysicalKey::Code(KeyCode::ArrowDown)) {
        player_input -= Vec2::Y;
    }

    if input.is_held(PhysicalKey::Code(KeyCode::ArrowRight)) {
        player_input -= Vec2::X;
    }

    if input.is_held(PhysicalKey::Code(KeyCode::ArrowLeft)) {
        player_input += Vec2::X;
    }

    for (mut movement, instance, ground, body_id) in movement.iter() {
        let Some(animation_player) = instance.animation_player() else {
            continue;
        };

        let Some(mut animation_player) = anim_players.get_entity(animation_player) else {
            continue;
        };

        let current_vel = movement.current_velocity();

        let just_pressed_jump = input.is_just_pressed(PhysicalKey::Code(KeyCode::Space));

        animation_player.set_bool_param("jumped", just_pressed_jump);

        animation_player.set_vec2_param(
            "movement",
            Vec2::new(-current_vel.x, current_vel.z) / PLAYER_SPEED,
        );

        let is_grounded = ground.is_grounded();

        if is_grounded && just_pressed_jump {
            physics.add_impulse(*body_id, Vec3::Y * 100000.0);
        }

        animation_player.set_bool_param("is_grounded", is_grounded);

        if is_grounded {
            movement
                .set_target_velocity(Vec3::new(player_input.x, 0.0, player_input.y) * PLAYER_SPEED);
        }
    }
}
