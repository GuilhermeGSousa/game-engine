use essential::assets::Asset;
use petgraph::graph::{DiGraph, NodeIndex};

use crate::node::{AnimationGraphNode, RootAnimationNode};

type AnimationDirectedGraph = DiGraph<Box<dyn AnimationGraphNode>, ()>;

#[derive(Asset)]
pub struct AnimationGraph {
    graph: AnimationDirectedGraph,
    root: NodeIndex,
}

impl AnimationGraph {
    pub fn new() -> Self {
        let mut graph = AnimationDirectedGraph::new();
        let root = graph.add_node(Box::new(RootAnimationNode));
        Self { graph, root }
    }
}
