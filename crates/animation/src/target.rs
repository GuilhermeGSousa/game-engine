use ecs::{component::Component, entity::Entity, query::Query, resource::Res};
use essential::{assets::asset_store::AssetStore, time::Time, transform::Transform};
use log::warn;
use uuid::Uuid;

use crate::{
    clip::AnimationClip,
    evaluation::{AnimationGraphEvaluationContext, AnimationGraphEvaluator},
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
            let context = AnimationGraphEvaluationContext {
                target_id: &animation_target.id,
                active_animation: animation_player.get_active_animation(&node_index),
                animation_clips: &animation_clips,
            };
            node.evaluate(context, &mut graph_evaluator);
        }

        // Now we just apply the root transform on the evaluator
        let Some(result_transform) = graph_evaluator.pop_transform() else {
            warn!("No result transform found for animation graph");
            continue;
        };

        target_transform.translation = result_transform.translation;
        target_transform.rotation = result_transform.rotation;
        target_transform.scale = result_transform.scale;
    }
}

pub(crate) fn update_animation_players(
    animation_players: Query<&mut AnimationPlayer>,
    animation_clips: Res<AssetStore<AnimationClip>>,
    time: Res<Time>,
) {
    let delta_time= time.delta().as_secs_f32();
    for mut animation_player in animation_players.iter() {
        for (_, active_animation) in animation_player.active_animations_mut().iter_mut() {

            let Some(anim_clip) = animation_clips.get(active_animation.current_animation()) else {
                return;
            };

            active_animation.update(delta_time, anim_clip.duration());
        }
    }
}
