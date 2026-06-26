use ecs::{
    query::{Query, query_filter::Changed},
    resource::Res,
};
use essential::{assets::asset_store::AssetStore, time::Time, transform::Transform};
use mesh::skeleton::SkeletonComponent;

use crate::{
    clip::AnimationClip,
    evaluation::AnimationGraphContext,
    graph::AnimationGraph,
    player::{AnimationHandleComponent, AnimationPlayer},
    root::AnimationRootBone,
};

pub(crate) fn animate_targets(
    animation_players: Query<(&mut AnimationPlayer, &SkeletonComponent)>,
    transforms: Query<&mut Transform>,
    root_bones: Query<&mut AnimationRootBone>,
    animation_graphs: Res<AssetStore<AnimationGraph>>,
    animation_clips: Res<AssetStore<AnimationClip>>,
) {
    for (mut animation_player, skeleton) in animation_players.iter() {
        animation_player.evaluate(
            &AnimationGraphContext {
                animation_clips: &animation_clips,
                animation_graphs: &animation_graphs,
            },
            skeleton.bone_ids(),
            skeleton.bones(),
            &transforms,
            &root_bones,
        );
    }
}

pub(crate) fn update_animation_players(
    animation_players: Query<&mut AnimationPlayer>,
    animation_clips: Res<AssetStore<AnimationClip>>,
    animation_graphs: Res<AssetStore<AnimationGraph>>,
    time: Res<Time>,
) {
    let delta_time = time.delta().as_secs_f32();
    for mut animation_player in animation_players.iter() {
        animation_player.update(
            delta_time,
            &AnimationGraphContext {
                animation_clips: &animation_clips,
                animation_graphs: &animation_graphs,
            },
        );
    }
}

pub(crate) fn initialize_animation_players(
    animation_players: Query<
        (&mut AnimationPlayer, &AnimationHandleComponent),
        Changed<AnimationHandleComponent>,
    >,
    animation_graphs: Res<AssetStore<AnimationGraph>>,
    animation_clips: Res<AssetStore<AnimationClip>>,
) {
    for (mut animation_player, graph_handle) in animation_players.iter() {
        animation_player.initialize_graph(
            (*graph_handle).clone(),
            &AnimationGraphContext {
                animation_clips: &animation_clips,
                animation_graphs: &animation_graphs,
            },
        );
    }
}
