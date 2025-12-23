use ecs::{
    component::Component,
    entity::Entity,
    query::{Query, query_filter::Changed},
    resource::Res,
};
use essential::{assets::asset_store::AssetStore, time::Time, transform::Transform};
use uuid::Uuid;

use crate::{
    clip::AnimationClip,
    evaluation::AnimationGraphContext,
    graph::AnimationGraph,
    player::{AnimationHandleComponent, AnimationPlayer},
};

#[derive(Component)]
pub struct AnimationTarget {
    pub id: Uuid,
    pub animator: Entity,
}

pub(crate) fn animate_targets(
    animation_players: Query<&AnimationPlayer>,
    animation_targets: Query<(&mut Transform, &AnimationTarget)>,
    animation_graphs: Res<AssetStore<AnimationGraph>>,
    animation_clips: Res<AssetStore<AnimationClip>>,
) {
    for (mut target_transform, animation_target) in animation_targets.iter() {
        let Some(animation_player) = animation_players.get_entity(animation_target.animator) else {
            continue;
        };

        let graph_instance = animation_player.graph_instance();

        **target_transform = graph_instance.evaluate(
            animation_target,
            &AnimationGraphContext {
                animation_clips: &animation_clips,
                animation_graphs: &animation_graphs,
            },
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
