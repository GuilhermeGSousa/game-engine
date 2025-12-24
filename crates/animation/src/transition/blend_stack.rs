use essential::transform::Transform;

use crate::{
    evaluation::AnimationGraphContext,
    graph::{AnimationGraphInstances, GraphId},
    target::AnimationTarget,
    transition::AnimationTransitionBlender,
};

pub(crate) struct BlendStack {
    current_graph: GraphId,
}

impl BlendStack {
    pub(crate) fn new(initial_graph: GraphId) -> Self {
        Self {
            current_graph: initial_graph,
        }
    }
}

impl AnimationTransitionBlender for BlendStack {
    fn sample(
        &self,
        target: &AnimationTarget,
        graph_instances: &AnimationGraphInstances,
        context: &AnimationGraphContext<'_>,
    ) -> essential::transform::Transform {
        graph_instances
            .get(self.current_graph)
            .map(|graph_instance| graph_instance.evaluate(target, context))
            .unwrap_or(Transform::IDENTITY)
    }

    fn update(
        &mut self,
        delta_time: f32,
        graph_instances: &mut AnimationGraphInstances,
        context: &AnimationGraphContext<'_>,
    ) {
        if let Some(graph_instance) = graph_instances.get_mut(self.current_graph) {
            graph_instance.update(delta_time, context);
        }
    }

    fn transition(
        &mut self,
        next_graph: GraphId,
        _graph_instances: &AnimationGraphInstances,
        _context: &AnimationGraphContext<'_>,
    ) {
        self.current_graph = next_graph;
    }
}
