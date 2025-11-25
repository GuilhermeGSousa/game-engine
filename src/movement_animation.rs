use animation::{
    clip::AnimationClip,
    graph::AnimationGraph,
    node::AnimationClipNode,
    player::{AnimationHandleComponent, AnimationPlayer},
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
            gltf_scenes.get(&loading_anim_store.idle),
            gltf_scenes.get(&loading_anim_store.walk),
        ) else {
            continue;
        };

        if let Some(anim_root) = spawned_gltf.animation_roots().first() {
            let anim_store = AnimationStore {
                idle: idle.animations()[0].clone(),
                walk: walk.animations()[0].clone(),
            };
            cmd.insert(anim_store, *anim_root);
            cmd.remove::<LoadingAnimationStore>(entity);
        }
    }
}

pub(crate) fn setup_animations(
    animation_roots: Query<
        (Entity, &mut AnimationPlayer, &AnimationStore),
        Without<AnimationHandleComponent>,
    >,
    gltf_comps: Query<&GLTFSpawnedMarker>,
    asset_server: Res<AssetServer>,
    mut cmd: CommandQueue,
) {
    for gltf_marker in gltf_comps.iter() {
        for anim_root in gltf_marker.animation_roots() {
            let Some((entity, mut animation_player, anim_store)) =
                animation_roots.get_entity(*anim_root)
            else {
                continue;
            };

            let mut anim_graph = AnimationGraph::new();

            // Add nodes
            let anim_clip_node = anim_graph.add_node(AnimationClipNode, *anim_graph.root());

            animation_player.start(&anim_clip_node, anim_store.idle.clone());

            cmd.insert(
                AnimationHandleComponent {
                    handle: asset_server.add(anim_graph),
                },
                entity,
            );
        }
    }
}

pub(crate) fn update_animation_state(
    animation_player: Query<(
        &mut AnimationPlayer,
        &AnimationStore,
        &AnimationHandleComponent,
    )>,
    anim_graphs: Res<AssetStore<AnimationGraph>>,
    input: Res<Input>,
) {
    let y_key_state = input.get_key_state(PhysicalKey::Code(KeyCode::KeyR));

    if y_key_state != InputState::Pressed {
        return;
    }

    for (mut player, anim_store, anim_handle) in animation_player.iter() {
        let Some(graph) = anim_graphs.get(&anim_handle) else {
            continue;
        };

        let Some(clip_node) = graph.get_node_inputs(*graph.root()).next() else {
            continue;
        };

        player.start(&clip_node, anim_store.walk.clone());
    }
}
