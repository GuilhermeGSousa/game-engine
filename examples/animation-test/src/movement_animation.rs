use color::LinearRgba;
use game_engine::{
    animation::{
        blackboard::AnimationBlackboard,
        clip::AnimationClip,
        graph::AnimationGraph,
        player::{AnimationHandleComponent, AnimationPlayer},
    },
    ecs::{
        command::CommandQueue,
        component::Component,
        entity::Entity,
        query::{query_filter::Without, Query},
        resource::Res,
    },
    essential::{
        assets::{asset_server::AssetServer, asset_store::AssetStore, handle::AssetHandle},
        transform::Transform,
    },
    gltf_loader::loader::{GLTFScene, GLTFSpawnerComponent, GLTFUsageSettings},
    render::components::{light::LightType, Light},
    window::input::Input,
};
use glam::{Quat, Vec2, Vec3};
use winit::keyboard::{KeyCode, PhysicalKey};

const GLB_ASSET: &str = "res/ninja/ninja.glb";
const IDLE_ANIM: &str = "res/ninja/idle.glb";
const WALK_ANIM: &str = "res/ninja/walk.glb";
const STRAFE_LEFT_ANIM: &str = "res/ninja/strafe_left.glb";
const STRAFE_RIGHT_ANIM: &str = "res/ninja/strafe_right.glb";

/// Marks the character entity spawned at startup (the GLTF spawner / animation player
/// entity), so the overlay and gizmo systems can find it.
#[derive(Component)]
pub(crate) struct AnimatedCharacter;

#[derive(Component)]
#[allow(dead_code)]
pub(crate) struct LoadingAnimationStore {
    pub(crate) idle: AssetHandle<GLTFScene>,
    pub(crate) walk: AssetHandle<GLTFScene>,
    pub(crate) strafe_left: AssetHandle<GLTFScene>,
    pub(crate) strafe_right: AssetHandle<GLTFScene>,
}

#[derive(Component)]
pub(crate) struct AnimationStore {
    pub(crate) idle: AssetHandle<AnimationClip>,
    pub(crate) walk: AssetHandle<AnimationClip>,
    pub(crate) strafe_left: AssetHandle<AnimationClip>,
    pub(crate) strafe_right: AssetHandle<AnimationClip>,
}

pub(crate) fn spawn_character(mut cmd: CommandQueue, asset_server: Res<AssetServer>) {
    let idle = asset_server.load::<GLTFScene>(IDLE_ANIM);
    let walk = asset_server.load::<GLTFScene>(WALK_ANIM);
    let strafe_left = asset_server.load::<GLTFScene>(STRAFE_LEFT_ANIM);
    let strafe_right = asset_server.load::<GLTFScene>(STRAFE_RIGHT_ANIM);
    let model = asset_server.load_with_usage_settings::<GLTFScene>(
        GLB_ASSET,
        GLTFUsageSettings {
            root_bone: Some("mixamorig:Hips"),
        },
    );

    cmd.spawn((
        AnimatedCharacter,
        GLTFSpawnerComponent(model),
        LoadingAnimationStore {
            idle,
            walk,
            strafe_left,
            strafe_right,
        },
        Transform::from_translation_rotation(Vec3::new(0.0, 0.0, -4.0), Quat::IDENTITY),
    ))
    .add_child((
        Light {
            color: LinearRgba::WHITE,
            intensity: 100.0,
            light_type: LightType::Point,
        },
        Transform::from_translation(Vec3::Y * 10.0),
    ));
}

pub(crate) fn setup_state_machine(
    animated_entities: Query<(Entity, &LoadingAnimationStore)>,
    gltf_scenes: Res<AssetStore<GLTFScene>>,
    mut cmd: CommandQueue,
) {
    for (entity, loading_anim_store) in animated_entities.iter() {
        let (Some(idle), Some(walk), Some(strafe_left), Some(strafe_right)) = (
            gltf_scenes
                .get(&loading_anim_store.idle)
                .and_then(|idle_scene| idle_scene.animations().first())
                .cloned(),
            gltf_scenes
                .get(&loading_anim_store.walk)
                .and_then(|walk_scene| walk_scene.animations().first())
                .cloned(),
            gltf_scenes
                .get(&loading_anim_store.strafe_left)
                .and_then(|strafe_left_scene| strafe_left_scene.animations().first())
                .cloned(),
            gltf_scenes
                .get(&loading_anim_store.strafe_right)
                .and_then(|strafe_right_scene| strafe_right_scene.animations().first())
                .cloned(),
        ) else {
            continue;
        };

        cmd.insert(
            AnimationStore {
                idle,
                walk,
                strafe_left,
                strafe_right,
            },
            entity,
        );
        cmd.remove::<LoadingAnimationStore>(entity);
    }
}

pub(crate) fn setup_animations(
    players: Query<(Entity, &AnimationPlayer), Without<AnimationHandleComponent>>,
    animation_stores: Query<&AnimationStore>,
    asset_server: Res<AssetServer>,
    mut cmd: CommandQueue,
) {
    let Some(anim_store) = animation_stores.iter().next() else {
        return;
    };

    for (player_entity, _player) in players.iter() {
        let mut movement_graph = AnimationGraph::new();

        movement_graph.result_node().with_blend_space_2d_input(
            |blackboard: &AnimationBlackboard| {
                blackboard.get_vec2("movement").unwrap_or(Vec2::ZERO)
            },
            |context| {
                context
                    .animation_clip_input(&anim_store.idle, Vec2::ZERO)
                    .animation_clip_input(&anim_store.strafe_left, Vec2::new(-1.0, 0.0))
                    .animation_clip_input(&anim_store.strafe_right, Vec2::new(1.0, 0.0))
                    .animation_clip_input(&anim_store.walk, Vec2::new(0.0, 1.0));
            },
        );

        cmd.insert(
            AnimationHandleComponent {
                handle: asset_server.add(movement_graph),
            },
            player_entity,
        );
    }
}

pub(crate) fn update_movement(anim_players: Query<&mut AnimationPlayer>, input: Res<Input>) {
    for mut anim_player in anim_players.iter() {
        let mut input_vec = Vec2::ZERO;

        if input.is_held(PhysicalKey::Code(KeyCode::ArrowUp)) {
            input_vec += Vec2::Y;
        }

        if input.is_held(PhysicalKey::Code(KeyCode::ArrowDown)) {
            input_vec -= Vec2::Y;
        }

        if input.is_held(PhysicalKey::Code(KeyCode::ArrowRight)) {
            input_vec += Vec2::X;
        }

        if input.is_held(PhysicalKey::Code(KeyCode::ArrowLeft)) {
            input_vec -= Vec2::X;
        }

        anim_player.set_vec2_param("movement", input_vec);
    }
}
