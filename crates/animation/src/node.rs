use std::any::Any;

use essential::{
    assets::handle::AssetHandle, blend::Blendable, transform::Transform, utils::AsAny,
};
use glam::{Quat, Vec3};

use crate::{
    clip::AnimationClip,
    evaluation::{
        AnimationGraphCreationContext, AnimationGraphEvaluationContext, AnimationGraphUpdateContext,
    },
    target::AnimationTarget,
};

pub trait AnimationNodeInstance: AsAny + Sync + Send {
    fn reset(&mut self);

    fn update(&mut self, context: AnimationGraphUpdateContext<'_>);
}

pub trait AnimationNode: AsAny + Sync + Send {
    fn create_instance(
        &self,
        _creation_context: &AnimationGraphCreationContext,
    ) -> Box<dyn AnimationNodeInstance>;
    fn evaluate(
        &self,
        target: &AnimationTarget,
        context: AnimationGraphEvaluationContext<'_>,
    ) -> Transform;
}

#[derive(AsAny)]
pub struct NoneInstance;

impl AnimationNodeInstance for NoneInstance {
    fn reset(&mut self) {}

    fn update(&mut self, _context: AnimationGraphUpdateContext<'_>) {}
}

#[derive(AsAny)]
pub struct AnimationRootNode;

impl AnimationNode for AnimationRootNode {
    fn evaluate(
        &self,
        _target: &AnimationTarget,
        context: AnimationGraphEvaluationContext<'_>,
    ) -> Transform {
        context
            .evaluated_inputs
            .first()
            .map(|evaluated_node| &evaluated_node.transform)
            .unwrap_or(&Transform::IDENTITY)
            .clone()
    }

    fn create_instance(
        &self,
        _creation_context: &AnimationGraphCreationContext,
    ) -> Box<dyn AnimationNodeInstance> {
        Box::new(NoneInstance)
    }
}

#[derive(AsAny)]
pub struct AnimationClipNodeInstance {
    time: f32,
    is_paused: bool,
    play_rate: f32,
}

impl AnimationClipNodeInstance {
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

impl AnimationNodeInstance for AnimationClipNodeInstance {
    fn reset(&mut self) {
        self.time = 0.0;
    }

    fn update(&mut self, context: AnimationGraphUpdateContext<'_>) {
        if self.is_paused {
            return;
        }

        let Some(clip_node) = context
            .animation_node
            .as_any()
            .downcast_ref::<AnimationClipNode>()
        else {
            return;
        };

        let Some(clip) = context.animation_clips.get(&clip_node.clip) else {
            return;
        };

        self.time += context.delta_time * self.play_rate;

        if self.time > clip.duration() {
            self.time = 0.0;
        }
    }
}

#[derive(AsAny)]
pub struct AnimationClipNode {
    clip: AssetHandle<AnimationClip>,
}

impl AnimationClipNode {
    pub fn new(clip: AssetHandle<AnimationClip>) -> Self {
        Self { clip }
    }
}

impl AnimationNode for AnimationClipNode {
    fn create_instance(
        &self,
        _creation_context: &AnimationGraphCreationContext,
    ) -> Box<dyn AnimationNodeInstance> {
        Box::new(AnimationClipNodeInstance::new())
    }

    fn evaluate(
        &self,
        target: &AnimationTarget,
        context: AnimationGraphEvaluationContext<'_>,
    ) -> Transform {
        let Some(animation_clip) = context.animation_clips.get(&self.clip) else {
            return Transform::IDENTITY;
        };

        // Find the channel for this animation target
        let Some(animation_channels) = animation_clip.get_channels(&target.id) else {
            return Transform::IDENTITY;
        };

        let Some(clip_anim_state) = context
            .current_node_state()
            .as_any()
            .downcast_ref::<AnimationClipNodeInstance>()
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
}

#[derive(AsAny)]
pub struct AnimationBlendNode;

impl AnimationNode for AnimationBlendNode {
    fn create_instance(
        &self,
        _creation_context: &AnimationGraphCreationContext,
    ) -> Box<dyn AnimationNodeInstance> {
        Box::new(NoneInstance)
    }

    fn evaluate(
        &self,
        _target: &AnimationTarget,
        context: AnimationGraphEvaluationContext<'_>,
    ) -> Transform {
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
