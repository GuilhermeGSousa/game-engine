use essential::assets::Asset;
use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::{Dfs, Walker},
};

use crate::node::{AnimationGraphNode, RootAnimationNode};

type AnimationDirectedGraph = DiGraph<Box<dyn AnimationGraphNode>, ()>;
type AnimationNodeIndex = NodeIndex;

#[derive(Asset)]
pub struct AnimationGraph {
    graph: AnimationDirectedGraph,
    root: AnimationNodeIndex,
}

impl AnimationGraph {
    pub fn new() -> Self {
        let mut graph = AnimationDirectedGraph::new();
        let root = graph.add_node(Box::new(RootAnimationNode));

        Self { graph, root }
    }

    pub fn add_node<T: AnimationGraphNode + 'static>(
        &mut self,
        node: T,
        parent_node: AnimationNodeIndex,
    ) -> AnimationNodeIndex {
        let added_node = self.graph.add_node(Box::new(node));
        self.graph.add_edge(parent_node, added_node, ());
        added_node
    }

    pub fn root(&self) -> &AnimationNodeIndex {
        &self.root
    }

    pub fn iter(&self) -> impl Iterator<Item = AnimationNodeIndex> + '_ {
        Dfs::new(&self.graph, self.root).iter(&self.graph)
    }

    pub fn get_node(&self, node_index: AnimationNodeIndex) -> Option<&Box<dyn AnimationGraphNode>> {
        self.graph.node_weight(node_index)
    }
}
