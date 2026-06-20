use crate::{
    evaluation::{AnimationGraphContext, AnimationGraphEvaluator},
    graph::{AnimationGraphInstances, GraphId},
    pose::{Pose, PoseLayout},
};

pub(crate) mod blend_stack;
pub(crate) mod inertialization_blender;

pub(crate) trait AnimationTransitionBlender {
    /// Samples the blender into `output`, a full-skeleton pose pre-seeded with the bind pose.
    fn sample(
        &self,
        layout: &PoseLayout,
        graph_instances: &AnimationGraphInstances,
        context: &AnimationGraphContext<'_>,
        evaluator: &mut AnimationGraphEvaluator,
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
