use std::any::Any;

use essential::{
    assets::{handle::AssetHandle},
    blend::Blendable,
    transform::Transform,
};
use glam::{Quat, Vec3};

use crate::{clip::AnimationClip, evaluation::AnimationGraphEvaluationContext};

pub trait AnimationNodeState: Any + Sync + Send {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn reset(&mut self);

    fn update(&mut self, delta_time: f32, clip_duration: f32);
}

pub trait AnimationNode: Sync + Send {
    fn create_state(&self) -> Box<dyn AnimationNodeState>;
    fn evaluate(&self, context: AnimationGraphEvaluationContext<'_>) -> Transform;
    fn animation_clip(&self) -> Option<&AssetHandle<AnimationClip>> { None }
}

pub struct NoneState;
impl AnimationNodeState for NoneState {
    fn reset(&mut self) {}

    fn update(&mut self, _delta_time: f32, _clip_duration: f32) {}

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
}

impl AnimationClipNodeState {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            is_paused: false,
            play_rate: 1.0,
        }
    }

    pub fn play(&mut self) {
        self.time = 0.0;
        self.is_paused = false;
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

    fn update(&mut self, delta_time: f32, clip_duration: f32) {
        if self.is_paused {
            return;
        }

        self.time += delta_time * self.play_rate;

        if self.time > clip_duration {
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
        Box::new(AnimationClipNodeState::new())
    }

    fn evaluate(&self, context: AnimationGraphEvaluationContext<'_>) -> Transform {
        let Some(animation_clip) = context.animation_clips.get(&self.clip) else {
            return Transform::IDENTITY;
        };

        // Find the channel for this animation target
        let Some(animation_channels) = animation_clip.get_channels(&context.target_id) else {
            return Transform::IDENTITY;
        };

        let Some(clip_anim_state) = context
            .current_node_state()
            .as_any()
            .downcast_ref::<AnimationClipNodeState>()
        else {
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

    fn animation_clip(&self) -> Option<&AssetHandle<AnimationClip>> { Some(&self.clip) }
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
