use essential::{
    assets::{asset_store::AssetStore},
    transform::Transform,
};

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
        _context: AnimationGraphEvaluationContext<'_>,
        _evaluator: &mut AnimationGraphEvaluator,
    ) {
    }
}

pub struct RootAnimationNode;

impl AnimationGraphNode for RootAnimationNode {}

pub struct AnimationClipNode;

impl AnimationGraphNode for AnimationClipNode {
    fn update_animation(
        &self,
        anim_clips: &AssetStore<AnimationClip>,
        active_animation: &mut ActiveAnimation,
        delta_time: f32,
    ) {
        let Some(anim_clip) = anim_clips.get(active_animation.current_animation()) else {
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