use std::collections::HashMap;

use essential::{assets::asset_store::AssetStore, transform::Transform};
use petgraph::graph::Neighbors;
use uuid::Uuid;

use crate::{clip::AnimationClip, graph::AnimationNodeIndex, player::ActiveAnimation};

pub struct AnimationGraphEvaluator {
    evaluation_state: HashMap<AnimationNodeIndex, Transform>,
}

impl AnimationGraphEvaluator {
    pub fn new() -> Self {
        Self {
            evaluation_state: HashMap::new(),
        }
    }

    pub fn get_transform(&mut self, index: &AnimationNodeIndex) -> &Transform {
        self.evaluation_state
            .entry(*index)
            .or_insert_with(Transform::identity)
    }

    pub fn get_transform_mut(&mut self, index: &AnimationNodeIndex) -> &mut Transform {
        self.evaluation_state
            .entry(*index)
            .or_insert_with(Transform::identity)
    }
}

pub struct AnimationGraphEvaluationContext<'a> {
    pub(crate) target_id: &'a Uuid,
    pub(crate) node_index: &'a AnimationNodeIndex,
    pub(crate) active_animation: Option<&'a ActiveAnimation>,
    pub(crate) animation_clips: &'a AssetStore<AnimationClip>,
    pub(crate) node_neighbors: Neighbors<'a, ()>,
}
