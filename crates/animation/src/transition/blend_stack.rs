use essential::transform::Transform;

use crate::{
    evaluation::AnimationGraphContext, graph::GraphId, transition::AnimationTransitionBlender,
};

pub(crate) struct BlendStack {}

impl BlendStack {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl AnimationTransitionBlender for BlendStack {
    fn sample(
        &self,
        graph_instances: &crate::graph::AnimationGraphInstances,
        context: &AnimationGraphContext<'_>,
    ) -> essential::transform::Transform {
        Transform::IDENTITY
    }

    fn update(&mut self, delta_time: f32, graph_instances: &crate::graph::AnimationGraphInstances) {
        todo!()
    }

    fn transition(
        &mut self,
        next_graph: GraphId,
        graph_instances: &crate::graph::AnimationGraphInstances,
    ) {
        todo!()
    }
}
