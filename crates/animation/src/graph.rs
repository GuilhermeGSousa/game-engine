use std::{collections::HashMap, ops::Deref};

use essential::assets::{Asset, handle::AssetHandle};
use log::warn;
use petgraph::{
    Direction::Outgoing,
    graph::{DiGraph, Neighbors, NodeIndex},
    visit::{Dfs, DfsPostOrder, Walker},
};
use uuid::Uuid;

use crate::{
    clip::AnimationClip,
    evaluation::{AnimationGraphContext, AnimationGraphEvaluator},
    node::{AnimationClipNode, AnimationNode, AnimationNodeInstance, AnimationRootNode},
    player::ActiveNodeInstance,
    pose::{EvaluatedPose, Pose, PosePool},
};

type AnimationDirectedGraph = DiGraph<Box<dyn AnimationNode>, ()>;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct AnimationNodeIndex(NodeIndex);

impl Deref for AnimationNodeIndex {
    type Target = NodeIndex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<NodeIndex> for AnimationNodeIndex {
    fn from(value: NodeIndex) -> Self {
        AnimationNodeIndex(value)
    }
}

#[derive(Asset)]
pub struct AnimationGraph {
    graph: AnimationDirectedGraph,
    root: AnimationNodeIndex,
}

impl AnimationGraph {
    pub fn new() -> Self {
        let mut graph = AnimationDirectedGraph::new();
        let root = graph.add_node(Box::new(AnimationRootNode));

        Self {
            graph,
            root: root.into(),
        }
    }

    pub fn from_clip(clip: AssetHandle<AnimationClip>) -> Self {
        let mut graph = Self::new();
        graph.add_node(AnimationClipNode::new(clip), *graph.root());
        graph
    }

    pub fn add_node<T: AnimationNode + 'static>(
        &mut self,
        node: T,
        parent_node: AnimationNodeIndex,
    ) -> AnimationNodeIndex {
        let added_node = self.graph.add_node(Box::new(node));
        self.graph.add_edge(*parent_node, added_node, ());
        added_node.into()
    }

    pub fn root(&self) -> &AnimationNodeIndex {
        &self.root
    }

    pub fn iter(&self) -> impl Iterator<Item = AnimationNodeIndex> + '_ {
        Dfs::new(&self.graph, *self.root)
            .iter(&self.graph)
            .map(|node_index| node_index.into())
    }

    pub fn iter_post_order(&self) -> impl Iterator<Item = AnimationNodeIndex> + '_ {
        DfsPostOrder::new(&self.graph, *self.root)
            .iter(&self.graph)
            .map(|node_index| node_index.into())
    }

    pub fn get_node(&self, node_index: AnimationNodeIndex) -> Option<&dyn AnimationNode> {
        self.graph.node_weight(*node_index).map(|node| node.deref())
    }

    pub fn get_node_inputs(&self, node_index: AnimationNodeIndex) -> Neighbors<'_, (), u32> {
        self.graph.neighbors_directed(*node_index, Outgoing)
    }
}

impl Default for AnimationGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
pub(crate) struct AnimationGraphInstance {
    graph_handle: Option<AssetHandle<AnimationGraph>>,
    graph_state: HashMap<AnimationNodeIndex, ActiveNodeInstance>,
}

impl AnimationGraphInstance {
    pub(crate) fn get_active_node_instance(
        &self,
        node_index: &AnimationNodeIndex,
    ) -> Option<&ActiveNodeInstance> {
        self.graph_state.get(node_index)
    }

    #[allow(dead_code)]
    pub(crate) fn get_instance<T: AnimationNodeInstance>(
        &self,
        node_index: &AnimationNodeIndex,
    ) -> Option<&T> {
        self.graph_state
            .get(node_index)
            .and_then(|node_state| node_state.node_instance.as_any().downcast_ref::<T>())
    }

    pub(crate) fn get_instance_mut<T: AnimationNodeInstance>(
        &mut self,
        node_index: &AnimationNodeIndex,
    ) -> Option<&mut T> {
        self.graph_state
            .get_mut(node_index)
            .and_then(|node_state| node_state.node_instance.as_any_mut().downcast_mut::<T>())
    }

    pub fn set_node_weight(&mut self, node_index: &AnimationNodeIndex, weight: f32) {
        if let Some(active_anim) = self.graph_state.get_mut(node_index) {
            active_anim.weight = weight;
        }
    }

    pub(crate) fn initialize(
        &mut self,
        animation_graph: AssetHandle<AnimationGraph>,
        creation_context: &AnimationGraphContext,
    ) {
        self.graph_state.clear();
        let Some(graph) = creation_context.animation_graphs.get(&animation_graph) else {
            self.graph_handle = None;
            return;
        };

        self.graph_handle = Some(animation_graph);

        for node_index in graph.iter() {
            let Some(anim_node) = graph.get_node(node_index) else {
                continue;
            };

            let node_instance = anim_node.create_instance(creation_context);
            self.graph_state.insert(
                node_index,
                ActiveNodeInstance {
                    node_instance,
                    weight: 1.0,
                },
            );
        }
    }

    pub(crate) fn reset(&mut self) {
        self.graph_state.iter_mut().for_each(|(_, node_state)| {
            node_state.node_instance.reset();
        });
    }

    pub(crate) fn update(&mut self, delta_time: f32, context: &AnimationGraphContext<'_>) {
        let Some(graph) = self.get_animation_graph(context) else {
            return;
        };

        self.graph_state
            .iter_mut()
            .for_each(|(node_index, node_state)| {
                let Some(node) = graph.get_node(*node_index) else {
                    return;
                };

                node_state.update(node, delta_time, context);
            });
    }

    pub(crate) fn evaluate(
        &self,
        context: &AnimationGraphContext<'_>,
        bone_ids: &[Uuid],
        pool: &mut PosePool,
        output_pose: &mut Pose,
    ) {
        let Some(graph) = self.get_animation_graph(context) else {
            return;
        };

        let mut graph_evaluator = AnimationGraphEvaluator::new();

        for node_index in graph.iter_post_order() {
            let Some(node) = graph.get_node(node_index) else {
                continue;
            };

            let Some(node_state) = self.get_active_node_instance(&node_index) else {
                warn!(
                    "No node state found for node, make sure the animation player has been correctly initialized"
                );
                continue;
            };

            let input_count = graph.get_node_inputs(node_index).count();

            let stack_start = graph_evaluator.stack_len() - input_count;

            let mut node_output_pose = pool.acquire();

            node_state.node_instance.evaluate(
                node,
                context,
                bone_ids,
                &graph_evaluator.view(stack_start),
                pool,
                &mut node_output_pose,
            );

            // Consume inputs and add them back to the free list
            for evaluated_pose in graph_evaluator.evaluation_stack.drain(stack_start..) {
                pool.release(evaluated_pose.pose);
            }

            graph_evaluator.push_evaluation(EvaluatedPose {
                pose: node_output_pose,
                weight: node_state.weight,
            });
        }

        let mut result = graph_evaluator
            .pop_evaluation()
            .map(|evaluated_pose| evaluated_pose.pose)
            .unwrap_or(Pose::identity(bone_ids.len()));

        std::mem::swap(output_pose, &mut result);
        pool.release(result);
    }

    pub(crate) fn get_animation_graph<'a>(
        &self,
        context: &'a AnimationGraphContext<'a>,
    ) -> Option<&'a AnimationGraph> {
        self.graph_handle
            .as_ref()
            .and_then(move |handle| context.animation_graphs().get(handle))
    }
}

#[derive(Clone, Copy)]
pub(crate) struct GraphId(usize);

impl Deref for GraphId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<usize> for GraphId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

pub(crate) struct AnimationGraphInstances {
    instances: Vec<AnimationGraphInstance>,
}

impl AnimationGraphInstances {
    pub(crate) fn new(instances: Vec<AnimationGraphInstance>) -> Self {
        Self { instances }
    }

    pub(crate) fn get(&self, id: GraphId) -> Option<&AnimationGraphInstance> {
        self.instances.get(*id)
    }

    pub(crate) fn get_mut(&mut self, id: GraphId) -> Option<&mut AnimationGraphInstance> {
        self.instances.get_mut(*id)
    }
}
