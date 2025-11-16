use std::ops::Deref;

use essential::assets::{asset_store::AssetStore, handle::AssetHandle};
use log::warn;

use crate::{
    clip::AnimationClip,
    evaluation::{AnimationGraphEvaluationContext, AnimationGraphEvaluator},
    player::ActiveAnimation,
};

pub trait AnimationGraphNode: Sync + Send {
    fn update_animation(
        &self,
        _anim_clips: &AssetStore<AnimationClip>,
        _active_animation: &mut ActiveAnimation,
        _delta_time: f32,
    ) {
    }

    fn evaluate(
        &self,
        context: AnimationGraphEvaluationContext<'_>,
        evaluator: &mut AnimationGraphEvaluator,
    );
}

pub struct RootAnimationNode;

impl AnimationGraphNode for RootAnimationNode {
    fn evaluate(
        &self,
        mut context: AnimationGraphEvaluationContext<'_>,
        evaluator: &mut AnimationGraphEvaluator,
    ) {
        let Some(input_node) = context.node_neighbors.next() else {
            warn!("No input node found for animation graph root node");
            return;
        };
        let input_transform = evaluator.get_transform(&input_node).clone();
        let result_transform = evaluator.get_transform_mut(context.node_index);
        result_transform.translation = input_transform.translation;
        result_transform.rotation = input_transform.rotation;
        result_transform.scale = input_transform.scale;
    }
}

pub struct AnimationClipNode(AssetHandle<AnimationClip>);

impl AnimationGraphNode for AnimationClipNode {
    fn update_animation(
        &self,
        anim_clips: &AssetStore<AnimationClip>,
        active_animation: &mut ActiveAnimation,
        delta_time: f32,
    ) {
        let Some(anim_clip) = anim_clips.get(&self) else {
            return;
        };

        active_animation.update(delta_time, anim_clip.duration());
    }

    fn evaluate(
        &self,
        context: AnimationGraphEvaluationContext<'_>,
        evaluator: &mut AnimationGraphEvaluator,
    ) {
        let Some(animation_state) = context.active_animation else {
            return;
        };

        let Some(animation_clip) = context.animation_clips.get(&self) else {
            return;
        };

        // Find the channel for this animation target
        let Some(animation_channels) = animation_clip.get_channels(&context.target_id) else {
            return;
        };

        // Based on the current time of the animation player + delta time, interpolate the target's transform
        let target_transform = evaluator.get_transform_mut(context.node_index);
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
