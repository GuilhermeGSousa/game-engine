use std::{collections::HashMap, ops::Deref};

use essential::assets::{Asset, asset_store::AssetStore};
use petgraph::{
    Direction::Outgoing,
    graph::{DiGraph, Neighbors, NodeIndex},
    visit::{Dfs, DfsPostOrder, Walker},
};

use crate::{
    clip::AnimationClip,
    evaluation::{AnimationGraphCreationContext, AnimationGraphUpdateContext},
    node::{AnimationNode, AnimationNodeInstance, AnimationRootNode},
    player::ActiveNodeInstance,
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

    pub fn get_node(&self, node_index: AnimationNodeIndex) -> Option<&Box<dyn AnimationNode>> {
        self.graph.node_weight(*node_index)
    }

    pub fn get_node_inputs(&self, node_index: AnimationNodeIndex) -> Neighbors<'_, (), u32> {
        self.graph.neighbors_directed(*node_index, Outgoing)
    }
}

#[derive(Default)]
pub(crate) struct AnimationGraphInstance(HashMap<AnimationNodeIndex, ActiveNodeInstance>);

impl AnimationGraphInstance {
    pub(crate) fn get_active_node_instance(
        &self,
        node_index: &AnimationNodeIndex,
    ) -> Option<&ActiveNodeInstance> {
        self.0.get(node_index)
    }

    #[allow(dead_code)]
    pub(crate) fn get_instance<T: AnimationNodeInstance>(
        &self,
        node_index: &AnimationNodeIndex,
    ) -> Option<&T> {
        self.0
            .get(node_index)
            .and_then(|node_state| node_state.node_instance.as_any().downcast_ref::<T>())
    }

    pub(crate) fn get_instance_mut<T: AnimationNodeInstance>(
        &mut self,
        node_index: &AnimationNodeIndex,
    ) -> Option<&mut T> {
        self.0
            .get_mut(node_index)
            .and_then(|node_state| node_state.node_instance.as_any_mut().downcast_mut::<T>())
    }

    pub fn set_node_weight(&mut self, node_index: &AnimationNodeIndex, weight: f32) {
        if let Some(active_anim) = self.0.get_mut(node_index) {
            active_anim.weight = weight;
        }
    }

    pub(crate) fn initialize(
        &mut self,
        animation_graph: &AnimationGraph,
        creation_context: &AnimationGraphCreationContext,
    ) {
        self.0.clear();
        for node_index in animation_graph.iter() {
            let Some(anim_node) = animation_graph.get_node(node_index) else {
                continue;
            };

            let node_instance = anim_node.create_instance(creation_context);
            self.0.insert(
                node_index,
                ActiveNodeInstance {
                    node_instance,
                    weight: 1.0,
                },
            );
        }
    }

    pub(crate) fn update(
        &mut self,
        delta_time: f32,
        graph: &AnimationGraph,
        animation_clips: &AssetStore<AnimationClip>,
        animation_graphs: &AssetStore<AnimationGraph>,
    ) {
        self.0.iter_mut().for_each(|(node_index, node_state)| {
            let Some(node) = graph.get_node(*node_index) else {
                return;
            };

            let context = AnimationGraphUpdateContext {
                animation_node: node,
                animation_clips,
                animation_graphs,
                delta_time,
            };

            node_state.update(context);
        });
    }
}
