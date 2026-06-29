use std::ops::Deref;

use ecs::{component::Component, entity::Entity, query::Query};
use essential::{assets::handle::AssetHandle, transform::Transform};
use log::info;
use uuid::Uuid;

use crate::{
    evaluation::AnimationGraphContext,
    graph::{AnimationGraph, AnimationGraphInstance, AnimationNodeIndex},
    node::{
        AnimationClipNodeInstance, AnimationNode, AnimationNodeInstance,
        state_machine::{AnimationFSMVariableType, AnimationStateMachineInstance},
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
    pose_pool: PosePool,
}

impl AnimationPlayer {
    pub fn new(bone_count: usize) -> Self {
        Self {
            graph_instance: AnimationGraphInstance::default(),
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

    pub fn set_fsm_param<T: Into<String>>(
        &mut self,
        node_index: &AnimationNodeIndex,
        param_name: T,
        param_value: AnimationFSMVariableType,
    ) {
        let Some(fsm_instance) = self
            .graph_instance
            .get_instance_mut::<AnimationStateMachineInstance>(node_index)
        else {
            info!("No animation node found when setting FSM parameters");
            return;
        };

        fsm_instance.set_param(param_name.into(), param_value);
    }

    /// Returns the name of the state machine's current state, if `node_index` refers to a
    /// state machine node that has been initialized.
    pub fn current_fsm_state(&self, node_index: &AnimationNodeIndex) -> Option<&str> {
        self.graph_instance
            .get_instance::<AnimationStateMachineInstance>(node_index)
            .map(AnimationStateMachineInstance::current_state_name)
    }

    pub(crate) fn initialize_graph(
        &mut self,
        animation_graph: AssetHandle<AnimationGraph>,
        context: &AnimationGraphContext,
    ) {
        self.graph_instance.initialize(animation_graph, context);
    }

    pub(crate) fn update(&mut self, delta_time: f32, context: &AnimationGraphContext) {
        self.graph_instance.update(delta_time, context);
    }

    pub(crate) fn evaluate(
        &mut self,
        context: &AnimationGraphContext,
        bone_ids: &[Uuid],
        bones: &[Entity],
        transforms: &Query<&mut Transform>,
        root_bones: &Query<&mut AnimationRootBone>,
    ) {
        let mut output_pose = self.pose_pool.acquire();
        self.graph_instance
            .evaluate(context, bone_ids, &mut self.pose_pool, &mut output_pose);

        for (bone_index, bone_entity) in bones.iter().enumerate() {
            let Some(joint_pose) = output_pose.get_joint_pose(bone_index) else {
                continue;
            };

            // Root bone: record its motion on the component and leave the bone at its bind
            // pose instead of animating it.
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
