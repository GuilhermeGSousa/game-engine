use std::any::Any;

use essential::{assets::handle::AssetHandle, transform::Transform};

use crate::{clip::AnimationClip, evaluation::AnimationGraphEvaluationContext};

pub trait AnimationNodeState: Any + Sync + Send {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn reset(&mut self);

    fn update(&mut self, delta_time: f32);
}

pub trait AnimationNode: Sync + Send {
    fn create_state(&self) -> Box<dyn AnimationNodeState>;
    fn evaluate(&self, context: AnimationGraphEvaluationContext<'_>) -> Transform;
}

pub struct NoneState;
impl AnimationNodeState for NoneState {
    fn reset(&mut self) {}

    fn update(&mut self, _delta_time: f32) {}

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
            .unwrap_or(&Transform::identity())
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
    animation_clip: Option<AssetHandle<AnimationClip>>,
}

impl AnimationClipNodeState {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            is_paused: false,
            play_rate: 1.0,
            animation_clip: None,
        }
    }

    pub fn play(&mut self, anim_clip: AssetHandle<AnimationClip>) {
        self.time = 0.0;
        self.is_paused = false;
        self.animation_clip = Some(anim_clip);
    }

    pub fn current_animation(&self) -> &Option<AssetHandle<AnimationClip>> {
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

    fn update(&mut self, delta_time: f32) {
        if self.is_paused {
            return;
        }

        self.time += delta_time * self.play_rate;

        // if self.time > self.clip_duration {
        //     self.time = 0.0;
        // }
    }
}

pub struct AnimationClipNode;

impl AnimationNode for AnimationClipNode {
    fn create_state(&self) -> Box<dyn AnimationNodeState> {
        Box::new(AnimationClipNodeState::new())
    }

    fn evaluate(&self, context: AnimationGraphEvaluationContext<'_>) -> Transform {
        let Some(clip_anim_state) = context
            .node_state
            .node_state
            .as_any()
            .downcast_ref::<AnimationClipNodeState>()
        else {
            return Transform::identity();
        };

        let Some(current_anim) = clip_anim_state.current_animation() else {
            return Transform::identity();
        };

        let Some(animation_clip) = context.animation_clips.get(current_anim) else {
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
        for evaluated_input in context.evaluated_inputs {
            // TODO: Blend nodes
        }
        Transform::identity()
    }
}

pub struct AnimationStateMachineNode;
