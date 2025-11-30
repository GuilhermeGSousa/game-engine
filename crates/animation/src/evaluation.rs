use essential::{assets::asset_store::AssetStore, transform::Transform};
use uuid::Uuid;

use crate::{clip::AnimationClip, player::ActiveNodeState};

pub struct AnimationGraphEvaluator {
    evaluation_stack: Vec<Transform>,
}

impl AnimationGraphEvaluator {
    pub fn new() -> Self {
        Self {
            evaluation_stack: Vec::new(),
        }
    }

    pub fn push_transform(&mut self, transform: Transform) {
        self.evaluation_stack.push(transform);
    }

    pub fn pop_transform(&mut self) -> Option<Transform> {
        self.evaluation_stack.pop()
    }
}

pub struct AnimationGraphEvaluationContext<'a> {
    pub(crate) target_id: &'a Uuid,
    pub(crate) active_animation: &'a ActiveNodeState,
    pub(crate) animation_clips: &'a AssetStore<AnimationClip>,
    pub(crate) input_transforms: &'a Vec<Transform>,
}
