use ecs::{component::Component, entity::Entity, query::Query, resource::Res};
use essential::{assets::asset_store::AssetStore, time::Time, transform::Transform};
use uuid::Uuid;

use crate::{
    clip::AnimationClip,
    evaluation::AnimationGraphEvaluator,
    graph::AnimationGraph,
    player::{AnimationHandleComponent, AnimationPlayer},
};

#[derive(Component)]
pub struct AnimationTarget {
    pub id: Uuid,
    pub animator: Entity,
}

pub(crate) fn animate_targets(
    animation_players: Query<(&AnimationPlayer, &AnimationHandleComponent)>,
    animation_targets: Query<(&mut Transform, &AnimationTarget)>,
    animation_graphs: Res<AssetStore<AnimationGraph>>,
    animation_clips: Res<AssetStore<AnimationClip>>,
) {
    for (mut target_transform, animation_target) in animation_targets.iter() {
        let Some((animation_player, animation_handle)) =
            animation_players.get_entity(animation_target.animator)
        else {
            continue;
        };

        let Some(animation_graph) = animation_graphs.get(&animation_handle) else {
            continue;
        };

        let mut graph_evaluator = AnimationGraphEvaluator::new();

        for node_index in animation_graph.iter_post_order() {
            let Some(node) = animation_graph.get_node(node_index) else {
                continue;
            };

            node.evaluate(
                &animation_target.id,
                &node_index,
                animation_player.get_active_animation(&node_index),
                &animation_clips,
                &mut graph_evaluator,
                animation_graph.neighbors(node_index),
            );
        }

        // Now we just apply the root transform on the evaluator
        let result_transform = graph_evaluator.get_transform(&animation_graph.root());
        target_transform.translation = result_transform.translation;
        target_transform.rotation = result_transform.rotation;
        target_transform.scale = result_transform.scale;
    }
}

pub(crate) fn update_animation_players(
    animation_players: Query<&mut AnimationPlayer>,
    time: Res<Time>,
) {
    for mut animation_player in animation_players.iter() {
        animation_player.update(time.delta().as_secs_f32());
    }
}
