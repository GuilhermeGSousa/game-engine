use essential::{assets::asset_store::AssetStore, transform::Transform};
use uuid::Uuid;

use crate::{clip::AnimationClip, player::ActiveNodeState};

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
    pub(crate) target_id: &'a Uuid,
    pub(crate) node_state: &'a ActiveNodeState,
    pub(crate) animation_clips: &'a AssetStore<AnimationClip>,
    pub(crate) evaluated_inputs: &'a Vec<EvaluatedNode>,
}
