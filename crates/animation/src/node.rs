use std::ops::Deref;

use essential::assets::{asset_store::AssetStore, handle::AssetHandle};
use log::warn;
use petgraph::graph::Neighbors;
use uuid::Uuid;

use crate::{
    clip::AnimationClip,
    evaluation::AnimationGraphEvaluator,
    graph::AnimationNodeIndex,
    player::{ActiveAnimation, AnimationPlayer},
};

pub trait AnimationGraphNode: Sync + Send {
    fn evaluate(
        &self,
        target_id: &Uuid,
        node_index: &AnimationNodeIndex,
        active_animation: Option<&ActiveAnimation>,
        animation_clips: &AssetStore<AnimationClip>,
        evaluator: &mut AnimationGraphEvaluator,
        node_neighbors: Neighbors<'_, ()>,
    );
}

pub struct RootAnimationNode;

impl AnimationGraphNode for RootAnimationNode {
    fn evaluate(
        &self,
        target_id: &Uuid,
        node_index: &AnimationNodeIndex,
        active_animation: Option<&ActiveAnimation>,
        animation_clips: &AssetStore<AnimationClip>,
        evaluator: &mut AnimationGraphEvaluator,
        mut node_neighbors: Neighbors<'_, ()>,
    ) {
        let Some(input_node) = node_neighbors.next() else {
            warn!("No input node found for animation graph root node");
            return;
        };
        let input_transform = evaluator.get_transform(&input_node).clone();
        let result_transform = evaluator.get_transform_mut(node_index);
        result_transform.translation = input_transform.translation;
        result_transform.rotation = input_transform.rotation;
        result_transform.scale = input_transform.scale;
    }
}

pub struct AnimationClipNode(AssetHandle<AnimationClip>);

impl AnimationGraphNode for AnimationClipNode {
    fn evaluate(
        &self,
        target_id: &Uuid,
        node_index: &AnimationNodeIndex,
        active_animation: Option<&ActiveAnimation>,
        animation_clips: &AssetStore<AnimationClip>,
        evaluator: &mut AnimationGraphEvaluator,
        node_neighbors: Neighbors<'_, ()>,
    ) {
        let Some(animation_state) = active_animation else {
            return;
        };

        let Some(animation_clip) = animation_clips.get(&self) else {
            return;
        };

        // Find the channel for this animation target
        let Some(animation_channels) = animation_clip.get_channels(&target_id) else {
            return;
        };

        // Based on the current time of the animation player + delta time, interpolate the target's transform
        let target_transform = evaluator.get_transform_mut(node_index);
        for animation_channel in animation_channels {
            animation_channel.sample_transform(animation_state.current_time(), target_transform);
        }
    }
}

impl Deref for AnimationClipNode {
    type Target = AssetHandle<AnimationClip>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AnimationClipNode {
    pub fn new(clip: AssetHandle<AnimationClip>) -> Self {
        Self(clip)
    }
}
