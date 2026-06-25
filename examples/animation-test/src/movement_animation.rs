use game_engine::{
    animation::{
        clip::AnimationClip,
        graph::{AnimationGraph, AnimationNodeIndex},
        player::{AnimationHandleComponent, AnimationPlayer},
        state_machine::{AnimationFSMTrigger, AnimationFSMVariableType, AnimationStateMachine},
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
    window::input::{Input, InputState},
};
use glam::{Quat, Vec3};
use winit::keyboard::{KeyCode, PhysicalKey};

const GLB_ASSET: &str = "res/girl.glb";
const IDLE_ANIM: &str = "res/idle.glb";
const WALK_ANIM: &str = "res/walk.glb";

/// Marks the character entity spawned at startup (the GLTF spawner / animation player
/// entity), so the overlay and gizmo systems can find it.
#[derive(Component)]
pub(crate) struct AnimatedCharacter;

#[derive(Component)]
pub(crate) struct LoadingAnimationStore {
    pub(crate) idle: AssetHandle<GLTFScene>,
    pub(crate) walk: AssetHandle<GLTFScene>,
}

#[derive(Component)]
pub(crate) struct AnimationStore {
    pub(crate) idle: AssetHandle<AnimationClip>,
    pub(crate) walk: AssetHandle<AnimationClip>,
}

#[derive(Component)]
pub(crate) struct AnimationFSMData {
    pub(crate) fsm_node: AnimationNodeIndex,
}

/// Startup system: immediately load the animated character and queue its animation clips.
pub(crate) fn spawn_character(mut cmd: CommandQueue, asset_server: Res<AssetServer>) {
    let idle_anim = asset_server.load::<GLTFScene>(IDLE_ANIM);
    let walk_anim = asset_server.load::<GLTFScene>(WALK_ANIM);
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
            idle: idle_anim,
            walk: walk_anim,
        },
        Transform::from_translation_rotation(Vec3::new(0.0, 0.0, -4.0), Quat::IDENTITY),
    ));
}

pub(crate) fn setup_state_machine(
    animated_entities: Query<(Entity, &LoadingAnimationStore)>,
    gltf_scenes: Res<AssetStore<GLTFScene>>,
    mut cmd: CommandQueue,
) {
    for (entity, loading_anim_store) in animated_entities.iter() {
        let (Some(idle), Some(walk)) = (
            gltf_scenes
                .get(&loading_anim_store.idle)
                .and_then(|idle_scene| idle_scene.animations().first())
                .cloned(),
            gltf_scenes
                .get(&loading_anim_store.walk)
                .and_then(|walk_scene| walk_scene.animations().first())
                .cloned(),
        ) else {
            continue;
        };

        cmd.insert(AnimationStore { idle, walk }, entity);
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
        let mut anim_graph = AnimationGraph::new();

        let anim_fsm = AnimationStateMachine::from_initial_state(
            "idle",
            asset_server.add(AnimationGraph::from_clip(anim_store.idle.clone())),
            |transition| {
                transition.to("walk", AnimationFSMTrigger::on_bool("has_moved", true), 0.5);
            },
        )
        .state(
            "walk",
            asset_server.add(AnimationGraph::from_clip(anim_store.walk.clone())),
            |transition| {
                transition.to(
                    "idle",
                    AnimationFSMTrigger::on_bool("has_moved", false),
                    0.5,
                );
            },
        )
        .build();

        let fsm_node = anim_graph.add_node(anim_fsm, *anim_graph.root());

        cmd.insert(
            AnimationHandleComponent {
                handle: asset_server.add(anim_graph),
            },
            player_entity,
        );
        cmd.insert(AnimationFSMData { fsm_node }, player_entity);
    }
}

pub(crate) fn update_movement_fsm(
    anim_players: Query<(&mut AnimationPlayer, &AnimationFSMData)>,
    input: Res<Input>,
) {
    for (mut anim_player, data) in anim_players.iter() {
        let key_state = input.get_key_state(PhysicalKey::Code(KeyCode::KeyO));

        if matches!(key_state, InputState::Pressed) {
            anim_player.set_fsm_param(
                &data.fsm_node,
                "has_moved",
                AnimationFSMVariableType::Bool(true),
            );
        } else if matches!(key_state, InputState::Released) {
            anim_player.set_fsm_param(
                &data.fsm_node,
                "has_moved",
                AnimationFSMVariableType::Bool(false),
            );
        }
    }
}
