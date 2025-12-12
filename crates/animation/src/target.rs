use ecs::{component::Component, entity::Entity, query::Query, resource::Res};
use essential::{assets::asset_store::AssetStore, time::Time, transform::Transform};
use log::warn;
use uuid::Uuid;

use crate::{
    clip::AnimationClip,
    evaluation::{AnimationGraphEvaluationContext, AnimationGraphEvaluator, EvaluatedNode},
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

            let Some(node_state) = animation_player.get_node_state(&node_index) else {
                warn!(
                    "No node state found for node, make sure the animation player has been correctly initialized"
                );
                continue;
            };

            let evaluated_inputs = animation_graph
                .get_node_inputs(node_index)
                .map(|_| graph_evaluator.pop_evaluation())
                .filter_map(|transform| transform)
                .collect::<Vec<_>>();

            let context = AnimationGraphEvaluationContext {
                target_id: &animation_target.id,
                node_state,
                animation_clips: &animation_clips,
                evaluated_inputs: &evaluated_inputs,
            };

            graph_evaluator.push_evaluation(EvaluatedNode {
                transform: node.evaluate(context),
                weight: node_state.weight,
            });
        }

        // Now we just apply the root transform on the evaluator
        let Some(result_transform) = graph_evaluator.pop_evaluation() else {
            warn!("No result transform found for animation graph");
            continue;
        };

        target_transform.translation = result_transform.transform.translation;
        target_transform.rotation = result_transform.transform.rotation;
        target_transform.scale = result_transform.transform.scale;
    }
}

pub(crate) fn update_animation_players(
    animation_players: Query<(&mut AnimationPlayer, &AnimationHandleComponent)>,
    animation_clips: Res<AssetStore<AnimationClip>>,
    animation_graphs: Res<AssetStore<AnimationGraph>>,
    time: Res<Time>,
) {
    let delta_time = time.delta().as_secs_f32();
    for (mut animation_player, graph_handle) in animation_players.iter() {

        let Some(graph) = animation_graphs.get(&graph_handle) else {
            continue;
        };

        animation_player.update(delta_time, graph, &animation_clips);
    }
}
