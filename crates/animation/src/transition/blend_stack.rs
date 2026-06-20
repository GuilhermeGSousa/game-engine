use crate::{
    evaluation::{AnimationGraphContext, AnimationGraphEvaluator},
    graph::{AnimationGraphInstances, GraphId},
    pose::{Pose, PoseLayout},
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
        layout: &PoseLayout,
        graph_instances: &AnimationGraphInstances,
        context: &AnimationGraphContext<'_>,
        evaluator: &mut AnimationGraphEvaluator,
        output: &mut Pose,
    ) {
        // Evaluate the active graph into the output pose (left at bind if it's missing).
        if let Some(graph_instance) = graph_instances.get(self.current_graph) {
            graph_instance.evaluate(layout, context, evaluator, output);
        }

        // Blend each transitioning layer on top, using a pooled temporary pose.
        for layer in &self.layers {
            let mut layer_pose = evaluator.acquire(layout);
            if let Some(graph_instance) = graph_instances.get(layer.graph_id) {
                graph_instance.evaluate(layout, context, evaluator, &mut layer_pose);
            }
            output.interpolate_into(&layer_pose, layer.weight);
            evaluator.release(layer_pose);
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
