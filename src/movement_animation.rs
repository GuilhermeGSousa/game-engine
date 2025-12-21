use std::collections::HashMap;

use animation::{
    clip::AnimationClip,
    graph::{AnimationGraph, AnimationNodeIndex},
    node::AnimationClipNode,
    player::{AnimationHandleComponent, AnimationPlayer},
    state_machine::{
        AnimationFSMStateDefinition, AnimationFSMTrigger, AnimationFSMVariableType,
        AnimationStateMachine, AnimationStateMachineTransitionDefinition,
    },
};
use ecs::{
    command::CommandQueue,
    component::Component,
    entity::Entity,
    query::{query_filter::Without, Query},
    resource::Res,
};
use essential::{
    assets::{asset_server::AssetServer, asset_store::AssetStore, handle::AssetHandle},
    transform::Transform,
};
use gltf::loader::{GLTFScene, GLTFSpawnedMarker, GLTFSpawnerComponent};
use render::components::camera::Camera;
use window::input::{Input, InputState};
use winit::keyboard::{KeyCode, PhysicalKey};

const GLB_ASSET: &str = "res/girl.glb";
const IDLE_ANIM: &str = "res/idle.glb";
const WALK_ANIM: &str = "res/walk.glb";

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

pub(crate) fn spawn_on_button_press(
    cameras: Query<(&Camera, &Transform)>,
    mut cmd: CommandQueue,
    input: Res<Input>,
    asset_server: Res<AssetServer>,
) {
    let (_, pos) = cameras.iter().next().expect("No camera found");
    let key_p = input.get_key_state(PhysicalKey::Code(KeyCode::KeyP));

    let mut mesh_transform = pos.clone();
    mesh_transform.translation = pos.translation + pos.forward() * 50.0;
    if key_p == InputState::Pressed {
        let idle_anim = asset_server.load::<GLTFScene>(IDLE_ANIM);
        let walk_anim = asset_server.load::<GLTFScene>(WALK_ANIM);

        cmd.spawn((
            GLTFSpawnerComponent(asset_server.load::<GLTFScene>(GLB_ASSET)),
            LoadingAnimationStore {
                idle: idle_anim,
                walk: walk_anim,
            },
            mesh_transform,
        ));
    }
}

pub(crate) fn setup_state_machine(
    animated_entities: Query<(Entity, &LoadingAnimationStore, &GLTFSpawnedMarker)>,
    gltf_scenes: Res<AssetStore<GLTFScene>>,
    mut cmd: CommandQueue,
) {
    for (entity, loading_anim_store, spawned_gltf) in animated_entities.iter() {
        let (Some(idle), Some(walk)) = (
            gltf_scenes
                .get(&loading_anim_store.idle)
                .and_then(|idle_scene| idle_scene.animations().first())
                .map(|clip| clip.clone()),
            gltf_scenes
                .get(&loading_anim_store.walk)
                .and_then(|walk_scene| walk_scene.animations().first())
                .map(|clip| clip.clone()),
        ) else {
            continue;
        };

        if let Some(anim_root) = spawned_gltf.animation_roots().first() {
            let anim_store: AnimationStore = AnimationStore { idle, walk };
            cmd.insert(anim_store, *anim_root);
            cmd.remove::<LoadingAnimationStore>(entity);
        }
    }
}

pub(crate) fn setup_animations(
    animation_roots: Query<(Entity, &AnimationStore), Without<AnimationHandleComponent>>,
    gltf_comps: Query<&GLTFSpawnedMarker>,
    asset_server: Res<AssetServer>,
    mut cmd: CommandQueue,
) {
    for gltf_marker in gltf_comps.iter() {
        for anim_root in gltf_marker.animation_roots() {
            let Some((entity, anim_store)) = animation_roots.get_entity(*anim_root) else {
                continue;
            };

            let mut anim_graph = AnimationGraph::new();
            let mut idle_graph = AnimationGraph::new();
            let mut walk_graph = AnimationGraph::new();

            idle_graph.add_node(
                AnimationClipNode::new(anim_store.idle.clone()),
                *idle_graph.root(),
            );

            walk_graph.add_node(
                AnimationClipNode::new(anim_store.walk.clone()),
                *walk_graph.root(),
            );

            let states_definition = vec![
                AnimationFSMStateDefinition {
                    name: "idle",
                    graph: asset_server.add(idle_graph),
                },
                AnimationFSMStateDefinition {
                    name: "walk",
                    graph: asset_server.add(walk_graph),
                },
            ];

            let transitions_definition = HashMap::from([
                (
                    "idle",
                    vec![AnimationStateMachineTransitionDefinition {
                        target_state: "walk",
                        trigger: AnimationFSMTrigger::from_condition(|params| {
                            params
                                .get("has_moved")
                                .map(|param| match param {
                                    animation::state_machine::AnimationFSMVariableType::Bool(
                                        val,
                                    ) => *val,
                                    _ => false,
                                })
                                .unwrap_or(false)
                        }),
                    }],
                ),
                (
                    "walk",
                    vec![AnimationStateMachineTransitionDefinition {
                        target_state: "idle",
                        trigger: AnimationFSMTrigger::from_condition(|params| {
                            params
                                .get("has_moved")
                                .map(|param| match param {
                                    animation::state_machine::AnimationFSMVariableType::Bool(
                                        val,
                                    ) => !*val,
                                    _ => false,
                                })
                                .unwrap_or(false)
                        }),
                    }],
                ),
            ]);
            let anim_fsm =
                AnimationStateMachine::new("idle", states_definition, transitions_definition);

            let fsm_node = anim_graph.add_node(anim_fsm, *anim_graph.root());

            cmd.insert(
                AnimationHandleComponent {
                    handle: asset_server.add(anim_graph),
                },
                entity,
            );

            cmd.insert(AnimationFSMData { fsm_node }, entity);
        }
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
