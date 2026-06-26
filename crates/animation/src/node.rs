use std::any::Any;

use essential::{assets::handle::AssetHandle, utils::AsAny};
use uuid::Uuid;

use crate::{
    clip::AnimationClip,
    evaluation::AnimationGraphContext,
    pose::{EvaluatedPose, Pose, PosePool},
};

pub trait AnimationNodeInstance: AsAny + Sync + Send {
    fn reset(&mut self);

    fn evaluate(
        &self,
        node: &dyn AnimationNode,
        context: &AnimationGraphContext<'_>,
        bone_ids: &[Uuid],
        evaluated_inputs: &[EvaluatedPose],
        pool: &mut PosePool,
        output: &mut Pose,
    );

    fn update(
        &mut self,
        node: &dyn AnimationNode,
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
        _node: &dyn AnimationNode,
        _delta_time: f32,
        _context: &AnimationGraphContext<'_>,
    ) {
    }

    fn evaluate(
        &self,
        _node: &dyn AnimationNode,
        _context: &AnimationGraphContext<'_>,
        _bone_ids: &[Uuid],
        evaluated_inputs: &[EvaluatedPose],
        _pool: &mut PosePool,
        output: &mut Pose,
    ) {
        // Pass-through: forward the first input pose if there is one, otherwise leave the
        // output at its acquired (identity) state.
        if let Some(input) = evaluated_inputs.first() {
            output.copy_from(&input.pose);
        }
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

impl Default for AnimationClipNodeInstance {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationNodeInstance for AnimationClipNodeInstance {
    fn reset(&mut self) {
        self.time = 0.0;
    }

    fn evaluate(
        &self,
        node: &dyn AnimationNode,
        context: &AnimationGraphContext<'_>,
        bone_ids: &[Uuid],
        _evaluated_inputs: &[EvaluatedPose],
        _pool: &mut PosePool,
        output: &mut Pose,
    ) {
        let Some(animation_clip) = node
            .as_any()
            .downcast_ref::<AnimationClipNode>()
            .and_then(|animation_clip| context.animation_clips.get(&animation_clip.clip))
        else {
            return;
        };

        bone_ids
            .iter()
            .map(|uuid| animation_clip.get_channels(uuid))
            .enumerate()
            .for_each(|(bone_index, animation_channels)| {
                let Some(animation_channels) = animation_channels else {
                    return;
                };

                let Some(joint_pose) = output.get_joint_pose_mut(bone_index) else {
                    return;
                };

                for animation_channel in animation_channels {
                    animation_channel.sample_transform(self.current_time(), joint_pose);
                }
            });
    }

    fn update(
        &mut self,
        node: &dyn AnimationNode,
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
