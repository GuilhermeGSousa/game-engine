use std::ops::Deref;

use ecs::{component::Component, entity::Entity, query::Query};
use essential::{
    assets::{asset_store::AssetStore, handle::AssetHandle},
    transform::Transform,
};
use glam::Vec2;
use uuid::Uuid;

use crate::{
    blackboard::{AnimationBlackboard, AnimationBlackboardValue},
    clip::AnimationClip,
    evaluation::AnimationGraphContext,
    graph::{AnimationGraph, AnimationGraphInstance, AnimationNodeIndex},
    node::{
        AnimationClipNodeInstance, AnimationNode, AnimationNodeInstance,
        state_machine::AnimationStateMachineInstance,
    },
    pose::PosePool,
    root::AnimationRootBone,
};

pub struct ActiveNodeInstance {
    pub weight: f32,
    pub(crate) node_instance: Box<dyn AnimationNodeInstance>,
}

impl ActiveNodeInstance {
    pub(crate) fn update(
        &mut self,
        node: &dyn AnimationNode,
        delta_time: f32,
        context: &AnimationGraphContext<'_>,
    ) {
        self.node_instance.update(node, delta_time, context);
    }
}

#[derive(Component)]
pub struct AnimationPlayer {
    graph_instance: AnimationGraphInstance,
    blackboard: AnimationBlackboard,
    pose_pool: PosePool,
}

impl AnimationPlayer {
    pub fn new(bone_count: usize) -> Self {
        Self {
            graph_instance: AnimationGraphInstance::default(),
            blackboard: AnimationBlackboard::default(),
            pose_pool: PosePool::new(bone_count),
        }
    }

    pub fn play(&mut self, node_index: &AnimationNodeIndex) {
        if let Some(anim_clip_instance) = self
            .graph_instance
            .get_instance_mut::<AnimationClipNodeInstance>(node_index)
        {
            anim_clip_instance.play();
        }
    }

    pub fn set_node_weight(&mut self, node_index: &AnimationNodeIndex, weight: f32) {
        self.graph_instance.set_node_weight(node_index, weight);
    }

    pub fn set_param(&mut self, key: impl Into<String>, value: AnimationBlackboardValue) {
        self.blackboard.set(key, value);
    }

    pub fn set_bool_param(&mut self, key: impl Into<String>, value: bool) {
        self.set_param(key, AnimationBlackboardValue::Bool(value));
    }

    pub fn set_int_param(&mut self, key: impl Into<String>, value: u32) {
        self.set_param(key, AnimationBlackboardValue::Int(value));
    }

    pub fn set_float_param(&mut self, key: impl Into<String>, value: f32) {
        self.set_param(key, AnimationBlackboardValue::Float(value));
    }

    pub fn set_vec2_param(&mut self, key: impl Into<String>, value: Vec2) {
        self.set_param(key, AnimationBlackboardValue::Vec2(value));
    }

    pub fn current_fsm_state(&self, node_index: &AnimationNodeIndex) -> Option<&str> {
        self.graph_instance
            .get_instance::<AnimationStateMachineInstance>(node_index)
            .map(AnimationStateMachineInstance::current_state_name)
    }

    pub(crate) fn initialize_graph(
        &mut self,
        animation_graph: AssetHandle<AnimationGraph>,
        clips: &AssetStore<AnimationClip>,
        graphs: &AssetStore<AnimationGraph>,
    ) {
        let context = AnimationGraphContext {
            animation_clips: clips,
            animation_graphs: graphs,
            blackboard: &self.blackboard,
        };
        self.graph_instance.initialize(animation_graph, &context);
    }

    pub(crate) fn update(
        &mut self,
        delta_time: f32,
        clips: &AssetStore<AnimationClip>,
        graphs: &AssetStore<AnimationGraph>,
    ) {
        let context = AnimationGraphContext {
            animation_clips: clips,
            animation_graphs: graphs,
            blackboard: &self.blackboard,
        };
        self.graph_instance.update(delta_time, &context);
    }

    pub(crate) fn evaluate(
        &mut self,
        clips: &AssetStore<AnimationClip>,
        graphs: &AssetStore<AnimationGraph>,
        bone_ids: &[Uuid],
        bones: &[Entity],
        transforms: &Query<&mut Transform>,
        root_bones: &Query<&mut AnimationRootBone>,
    ) {
        let context = AnimationGraphContext {
            animation_clips: clips,
            animation_graphs: graphs,
            blackboard: &self.blackboard,
        };

        let mut output_pose = self.pose_pool.acquire();
        self.graph_instance
            .evaluate(&context, bone_ids, &mut self.pose_pool, &mut output_pose);

        for (bone_index, bone_entity) in bones.iter().enumerate() {
            let Some(joint_pose) = output_pose.get_joint_pose(bone_index) else {
                continue;
            };

            if let Some(mut root_bone) = root_bones.get_entity(*bone_entity) {
                root_bone.displacement = joint_pose.translation;
                continue;
            }

            if let Some(mut transform) = transforms.get_entity(*bone_entity) {
                transform.translation = joint_pose.translation;
                transform.rotation = joint_pose.rotation;
                transform.scale = joint_pose.scale;
            }
        }

        self.pose_pool.release(output_pose);
    }
}

#[derive(Component)]
pub struct AnimationHandleComponent {
    pub handle: AssetHandle<AnimationGraph>,
}

impl Deref for AnimationHandleComponent {
    type Target = AssetHandle<AnimationGraph>;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
