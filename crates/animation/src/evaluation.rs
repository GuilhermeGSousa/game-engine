use essential::{assets::asset_store::AssetStore, transform::Transform};

use crate::{clip::AnimationClip, graph::AnimationGraph};

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
    pub(crate) animation_clips: &'a AssetStore<AnimationClip>,
    pub(crate) animation_graphs: &'a AssetStore<AnimationGraph>,
}

impl<'a> AnimationGraphEvaluationContext<'a> {
    pub fn animation_clips(&self) -> &AssetStore<AnimationClip> {
        self.animation_clips
    }

    pub fn animation_graphs(&self) -> &AssetStore<AnimationGraph> {
        self.animation_graphs
    }
}
