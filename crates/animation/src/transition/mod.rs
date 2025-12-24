use essential::transform::Transform;

use crate::{
    evaluation::AnimationGraphContext,
    graph::{AnimationGraphInstances, GraphId},
    target::AnimationTarget,
};

pub(crate) mod blend_stack;
pub(crate) mod inertialization_blender;

pub(crate) trait AnimationTransitionBlender {
    fn sample(
        &self,
        target: &AnimationTarget,
        graph_instances: &AnimationGraphInstances,
        context: &AnimationGraphContext<'_>,
    ) -> Transform;

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
        context: &AnimationGraphContext<'_>,
    );
}
