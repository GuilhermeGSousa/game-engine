use essential::{
    transform::Transform,
};

use crate::{
    evaluation::{AnimationGraphEvaluationContext, AnimationGraphEvaluator},
};

pub trait AnimationGraphNode: Sync + Send {
    fn evaluate(
        &self,
        _context: AnimationGraphEvaluationContext<'_>,
        _evaluator: &mut AnimationGraphEvaluator,
    ) {
    }
}

pub struct RootAnimationNode;

impl AnimationGraphNode for RootAnimationNode {}

pub struct AnimationClipNode;

impl AnimationGraphNode for AnimationClipNode {
    fn evaluate(
        &self,
        context: AnimationGraphEvaluationContext<'_>,
        evaluator: &mut AnimationGraphEvaluator,
    ) {
        let Some(animation_state) = context.active_animation else {
            return;
        };

        let Some(animation_clip) = context.animation_clips.get(animation_state.current_animation()) else {
            return;
        };

        // Find the channel for this animation target
        let Some(animation_channels) = animation_clip.get_channels(&context.target_id) else {
            return;
        };

        // Based on the current time of the animation player + delta time, interpolate the target's transform
        let mut target_transform = Transform::identity();
        for animation_channel in animation_channels {
            animation_channel
                .sample_transform(animation_state.current_time(), &mut target_transform);
        }

        evaluator.push_transform(target_transform);
    }
}