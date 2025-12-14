use std::ops::Deref;

use essential::assets::Asset;
use petgraph::{
    Direction::Outgoing,
    graph::{DiGraph, Neighbors, NodeIndex},
    visit::{Dfs, DfsPostOrder, Walker},
};

use crate::node::{AnimationNode, AnimationRootNode};

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
