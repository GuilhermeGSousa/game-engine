use std::collections::HashMap;

use essential::transform::Transform;

use crate::graph::AnimationNodeIndex;

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
