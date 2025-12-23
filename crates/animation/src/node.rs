use std::any::Any;

use essential::{assets::handle::AssetHandle, transform::Transform, utils::AsAny};

use crate::{
    clip::AnimationClip,
    evaluation::{AnimationGraphContext, EvaluatedNode},
    target::AnimationTarget,
};

pub trait AnimationNodeInstance: AsAny + Sync + Send {
    fn reset(&mut self);

    fn evaluate(
        &self,
        node: &Box<dyn AnimationNode>,
        target: &AnimationTarget,
        evaluated_inputs: &Vec<EvaluatedNode>,
        context: AnimationGraphContext<'_>,
    ) -> Transform;

    fn update(
        &mut self,
        node: &Box<dyn AnimationNode>,
        delta_time: f32,
        context: &AnimationGraphContext<'_>,
    );
}

pub trait AnimationNode: AsAny + Sync + Send {
    fn create_instance(
        &self,
        _creation_context: &AnimationGraphContext,
    ) -> Box<dyn AnimationNodeInstance>;
}

#[derive(AsAny)]
pub struct NoneInstance;

impl AnimationNodeInstance for NoneInstance {
    fn reset(&mut self) {}

    fn update(
        &mut self,
        _node: &Box<dyn AnimationNode>,
        _delta_time: f32,
        _context: &AnimationGraphContext<'_>,
    ) {
    }

    fn evaluate(
        &self,
        _node: &Box<dyn AnimationNode>,
        _target: &AnimationTarget,
        evaluated_inputs: &Vec<EvaluatedNode>,
        _context: AnimationGraphContext<'_>,
    ) -> Transform {
        evaluated_inputs
            .first()
            .map(|evaluated_node| &evaluated_node.transform)
            .unwrap_or(&Transform::IDENTITY)
            .clone()
    }
}

#[derive(AsAny)]
pub struct AnimationRootNode;

impl AnimationNode for AnimationRootNode {
    fn create_instance(
        &self,
        _creation_context: &AnimationGraphContext,
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

    fn evaluate(
        &self,
        node: &Box<dyn AnimationNode>,
        target: &AnimationTarget,
        _evaluated_inputs: &Vec<EvaluatedNode>,
        context: AnimationGraphContext<'_>,
    ) -> Transform {
        let Some(animation_clip) = node
            .as_any()
            .downcast_ref::<AnimationClipNode>()
            .and_then(|animation_clip| context.animation_clips.get(&animation_clip.clip))
        else {
            return Transform::IDENTITY;
        };

        // Find the channel for this animation target
        let Some(animation_channels) = animation_clip.get_channels(&target.id) else {
            return Transform::IDENTITY;
        };

        // Based on the current time of the animation player + delta time, interpolate the target's transform
        let mut target_transform = Transform::IDENTITY;
        for animation_channel in animation_channels {
            animation_channel.sample_transform(self.current_time(), &mut target_transform);
        }

        target_transform
    }

    fn update(
        &mut self,
        node: &Box<dyn AnimationNode>,
        delta_time: f32,
        context: &AnimationGraphContext<'_>,
    ) {
        if self.is_paused {
            return;
        }

        let Some(clip_node) = node.as_any().downcast_ref::<AnimationClipNode>() else {
            return;
        };

        let Some(clip) = context.animation_clips.get(&clip_node.clip) else {
            return;
        };

        self.time += delta_time * self.play_rate;

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
        _creation_context: &AnimationGraphContext,
    ) -> Box<dyn AnimationNodeInstance> {
        Box::new(AnimationClipNodeInstance::new())
    }
}

#[derive(AsAny)]
pub struct AnimationBlendNode;

impl AnimationNode for AnimationBlendNode {
    fn create_instance(
        &self,
        _creation_context: &AnimationGraphContext,
    ) -> Box<dyn AnimationNodeInstance> {
        Box::new(NoneInstance)
    }
}
pub struct AnimationStateMachineNodeState;

pub struct AnimationStateMachineNode;
