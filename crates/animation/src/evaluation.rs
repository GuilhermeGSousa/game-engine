use essential::{assets::asset_store::AssetStore, transform::Transform};

use crate::{
    clip::AnimationClip,
    graph::AnimationGraph,
    node::{AnimationNode, AnimationNodeInstance},
    player::ActiveNodeInstance,
};

pub struct EvaluatedNode {
    pub transform: Transform,
    pub weight: f32,
}

pub struct AnimationGraphEvaluator {
    evaluation_stack: Vec<EvaluatedNode>,
}

impl AnimationGraphEvaluator {
    pub fn new() -> Self {
        Self {
            evaluation_stack: Vec::new(),
        }
    }

    pub fn push_evaluation(&mut self, evaluated_node: EvaluatedNode) {
        self.evaluation_stack.push(evaluated_node);
    }

    pub fn pop_evaluation(&mut self) -> Option<EvaluatedNode> {
        self.evaluation_stack.pop()
    }
}

pub struct AnimationGraphEvaluationContext<'a> {
    pub(crate) node_instance: &'a ActiveNodeInstance,
    pub(crate) animation_clips: &'a AssetStore<AnimationClip>,
    pub(crate) animation_graphs: &'a AssetStore<AnimationGraph>,
    pub(crate) evaluated_inputs: &'a Vec<EvaluatedNode>,
}

impl<'a> AnimationGraphEvaluationContext<'a> {
    pub fn current_node_state(&self) -> &Box<dyn AnimationNodeInstance> {
        &self.node_instance.node_instance
    }

    pub fn current_node_weight(&self) -> f32 {
        self.node_instance.weight
    }

    pub fn animation_clips(&self) -> &AssetStore<AnimationClip> {
        self.animation_clips
    }

    pub fn animation_graphs(&self) -> &AssetStore<AnimationGraph> {
        self.animation_graphs
    }
}

pub struct AnimationGraphUpdateContext<'a> {
    pub(crate) animation_node: &'a Box<dyn AnimationNode>,
    pub(crate) delta_time: f32,
    pub(crate) animation_clips: &'a AssetStore<AnimationClip>,
    pub(crate) animation_graphs: &'a AssetStore<AnimationGraph>,
}

#[allow(unused)]
pub struct AnimationGraphCreationContext<'a> {
    pub(crate) animation_clips: &'a AssetStore<AnimationClip>,
    pub(crate) animation_graphs: &'a AssetStore<AnimationGraph>,
}
