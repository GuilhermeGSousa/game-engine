use std::any::Any;

use essential::{
    assets::{asset_store::AssetStore, handle::AssetHandle},
    blend::Blendable,
    transform::Transform,
};
use glam::{Quat, Vec3};

use crate::{clip::AnimationClip, evaluation::AnimationGraphEvaluationContext};

pub trait AnimationNodeState: Any + Sync + Send {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn reset(&mut self);

    fn update(&mut self, delta_time: f32, animation_clips: &AssetStore<AnimationClip>);
}

pub trait AnimationNode: Sync + Send {
    fn create_state(&self) -> Box<dyn AnimationNodeState>;
    fn evaluate(&self, context: AnimationGraphEvaluationContext<'_>) -> Transform;
}

pub struct NoneState;
impl AnimationNodeState for NoneState {
    fn reset(&mut self) {}

    fn update(&mut self, _delta_time: f32, _animation_clips: &AssetStore<AnimationClip>) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct AnimationRootNode;

impl AnimationNode for AnimationRootNode {
    fn evaluate(&self, context: AnimationGraphEvaluationContext<'_>) -> Transform {
        context
            .evaluated_inputs
            .first()
            .map(|evaluated_node| &evaluated_node.transform)
            .unwrap_or(&Transform::IDENTITY)
            .clone()
    }

    fn create_state(&self) -> Box<dyn AnimationNodeState> {
        Box::new(NoneState)
    }
}

pub struct AnimationClipNodeState {
    time: f32,
    is_paused: bool,
    play_rate: f32,
    animation_clip: AssetHandle<AnimationClip>,
}

impl AnimationClipNodeState {
    pub fn new(clip: AssetHandle<AnimationClip>) -> Self {
        Self {
            time: 0.0,
            is_paused: false,
            play_rate: 1.0,
            animation_clip: clip,
        }
    }

    pub fn play(&mut self, anim_clip: AssetHandle<AnimationClip>) {
        self.time = 0.0;
        self.is_paused = false;
        self.animation_clip = anim_clip;
    }

    pub fn current_animation(&self) -> &AssetHandle<AnimationClip> {
        &self.animation_clip
    }

    pub fn current_time(&self) -> f32 {
        self.time
    }
}

impl AnimationNodeState for AnimationClipNodeState {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn reset(&mut self) {
        self.time = 0.0;
        self.play_rate = 1.0;
        self.is_paused = false;
    }

    fn update(&mut self, delta_time: f32, animation_clips: &AssetStore<AnimationClip>) {
        let Some(clip) = animation_clips.get(&self.animation_clip) else {
            return;
        };

        if self.is_paused {
            return;
        }

        self.time += delta_time * self.play_rate;

        if self.time > clip.duration() {
            self.time = 0.0;
        }
    }
}

pub struct AnimationClipNode {
    clip: AssetHandle<AnimationClip>,
}

impl AnimationClipNode {
    pub fn new(clip: AssetHandle<AnimationClip>) -> Self {
        Self { clip }
    }
}

impl AnimationNode for AnimationClipNode {
    fn create_state(&self) -> Box<dyn AnimationNodeState> {
        let clip = self.clip.clone();
        Box::new(AnimationClipNodeState::new(clip))
    }

    fn evaluate(&self, context: AnimationGraphEvaluationContext<'_>) -> Transform {
        let Some(clip_anim_state) = context
            .current_node_state()
            .as_any()
            .downcast_ref::<AnimationClipNodeState>()
        else {
            return Transform::IDENTITY;
        };

        let Some(animation_clip) = context
            .animation_clips
            .get(clip_anim_state.current_animation())
        else {
            return Transform::IDENTITY;
        };

        // Find the channel for this animation target
        let Some(animation_channels) = animation_clip.get_channels(&context.target_id) else {
            return Transform::IDENTITY;
        };

        // Based on the current time of the animation player + delta time, interpolate the target's transform
        let mut target_transform = Transform::IDENTITY;
        for animation_channel in animation_channels {
            animation_channel
                .sample_transform(clip_anim_state.current_time(), &mut target_transform);
        }

        target_transform
    }
}

pub struct AnimationBlendNode;

impl AnimationNode for AnimationBlendNode {
    fn create_state(&self) -> Box<dyn AnimationNodeState> {
        Box::new(NoneState)
    }

    fn evaluate(&self, context: AnimationGraphEvaluationContext<'_>) -> Transform {
        let mut translation = Vec3::ZERO;
        let mut rotation = Quat::IDENTITY;
        let mut scale = Vec3::ZERO;
        for evaluated_input in context.evaluated_inputs {
            translation += evaluated_input.transform.translation * evaluated_input.weight;
            rotation = Quat::interpolate(
                Quat::IDENTITY,
                evaluated_input.transform.rotation,
                evaluated_input.weight,
            ) * rotation;
            scale += evaluated_input.transform.scale * evaluated_input.weight;
        }

        Transform {
            translation,
            rotation,
            scale,
        }
    }
}
pub struct AnimationStateMachineNodeState;

pub struct AnimationStateMachineNode;
