use essential::{
    transform::Transform,
};

use crate::{
    evaluation::{AnimationGraphEvaluationContext},
};

pub trait AnimationGraphNode: Sync + Send {
    fn evaluate(
        &self,
        context: AnimationGraphEvaluationContext<'_>,
    ) -> Transform;
}

pub struct AnimationRootNode;

impl AnimationGraphNode for AnimationRootNode {
    fn evaluate(
        &self,
        context: AnimationGraphEvaluationContext<'_>,
    ) -> Transform
    {
        context.input_transforms.first().unwrap_or(&Transform::identity()).clone()
    }
}

pub struct AnimationClipNode;

impl AnimationGraphNode for AnimationClipNode {
    fn evaluate(
        &self,
        context: AnimationGraphEvaluationContext<'_>,
    ) -> Transform {
        let Some(animation_state) = context.active_animation else {
            return Transform::identity();
        };

        let Some(animation_clip) = context.animation_clips.get(animation_state.current_animation()) else {
            return Transform::identity();
        };

        // Find the channel for this animation target
        let Some(animation_channels) = animation_clip.get_channels(&context.target_id) else {
            return Transform::identity();
        };

        // Based on the current time of the animation player + delta time, interpolate the target's transform
        let mut target_transform = Transform::identity();
        for animation_channel in animation_channels {
            animation_channel
                .sample_transform(animation_state.current_time(), &mut target_transform);
        }

        target_transform
    }
}

pub struct AnimationBlendNode;

impl AnimationGraphNode for AnimationBlendNode
{
    fn evaluate(
        &self,
        _context: AnimationGraphEvaluationContext<'_>,
    ) -> Transform
    {
        Transform::identity()
    }

}