
use crate::{
    evaluation::AnimationGraphContext, graph::{AnimationGraphInstances, GraphId}, player::AnimationSkeletonBinding, pose::Pose
};

pub(crate) mod blend_stack;
pub(crate) mod inertialization_blender;

pub(crate) trait AnimationTransitionBlender {
    fn sample(
        &self,
        binding: &AnimationSkeletonBinding,
        graph_instances: &AnimationGraphInstances,
        context: &AnimationGraphContext<'_>,
        output: &mut Pose,
    );

    fn update(
        &mut self,
        delta_time: f32,
        graph_instances: &mut AnimationGraphInstances,
        context: &AnimationGraphContext<'_>,
    );

    fn transition(
        &mut self,
        next_graph: GraphId,
        graph_instances: &AnimationGraphInstances,
        transition_time: f32,
        context: &AnimationGraphContext<'_>,
    );
}
