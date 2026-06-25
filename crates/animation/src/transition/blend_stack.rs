use crate::{
    evaluation::AnimationGraphContext,
    graph::{AnimationGraphInstances, GraphId},
    player::AnimationSkeletonBinding,
    pose::{Pose, PosePool},
    transition::AnimationTransitionBlender,
};

struct BlendStackEntry {
    graph_id: GraphId,
    fade_speed: f32,
    weight: f32,
}

pub(crate) struct BlendStack {
    current_graph: GraphId,
    layers: Vec<BlendStackEntry>,
}

impl BlendStack {
    pub(crate) fn new(initial_graph: GraphId) -> Self {
        Self {
            current_graph: initial_graph,
            layers: Vec::new(),
        }
    }
}

impl AnimationTransitionBlender for BlendStack {
    fn sample(
        &self,
        binding: &AnimationSkeletonBinding,
        graph_instances: &AnimationGraphInstances,
        context: &AnimationGraphContext<'_>,
        pool: &mut PosePool,
        output: &mut Pose,
    ) {
        // Evaluate the current graph straight into the output pose.
        if let Some(graph_instance) = graph_instances.get(self.current_graph) {
            graph_instance.evaluate(context, binding, pool, output);
        }

        // Cross-fade each in-progress layer on top, by its current weight.
        for layer in &self.layers {
            let mut layer_pose = pool.acquire();

            if let Some(graph_instance) = graph_instances.get(layer.graph_id) {
                graph_instance.evaluate(context, binding, pool, &mut layer_pose);
            }

            output.blend(&layer_pose, layer.weight);
            pool.release(layer_pose);
        }
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

        self.layers.retain_mut(|entry| {
            let Some(graph_instance) = graph_instances.get_mut(entry.graph_id) else {
                return false;
            };

            graph_instance.update(delta_time, context);
            entry.weight = (entry.weight + entry.fade_speed * delta_time).min(1.0);

            if entry.weight >= 1.0 {
                if let Some(graph_instance) = graph_instances.get_mut(self.current_graph) {
                    graph_instance.reset();
                }
                self.current_graph = entry.graph_id;
                false
            } else {
                true
            }
        });
    }

    fn transition(
        &mut self,
        next_graph: GraphId,
        _graph_instances: &AnimationGraphInstances,
        transition_time: f32,
        _context: &AnimationGraphContext<'_>,
    ) {
        self.layers.push(BlendStackEntry {
            graph_id: next_graph,
            fade_speed: 1.0 / transition_time,
            weight: 0.0,
        });
    }
}
