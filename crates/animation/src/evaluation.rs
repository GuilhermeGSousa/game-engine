use essential::assets::asset_store::AssetStore;

use crate::{
    clip::AnimationClip,
    graph::AnimationGraph,
    pose::{Pose, PoseLayout},
};

/// A pose produced by a node, together with its blend weight.  Lives on the evaluator's
/// traversal stack while a graph is being evaluated.
pub struct EvaluatedPose {
    pub pose: Pose,
    pub weight: f32,
}

/// Scratch state for evaluating animation graphs into full-skeleton poses.
///
/// Owns a post-order traversal stack and a pool of reusable [`Pose`] buffers so that a whole
/// player (including nested state-machine sub-graphs) can be evaluated each frame without
/// per-node or per-frame heap allocation.  One evaluator is stored per [`AnimationPlayer`]
/// so its capacity persists across frames.
#[derive(Default)]
pub struct AnimationGraphEvaluator {
    pub(crate) stack: Vec<EvaluatedPose>,
    pool: Vec<Pose>,
    pub(crate) inputs_scratch: Vec<Pose>,
}

impl AnimationGraphEvaluator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrows a pose buffer from the pool (or allocates one) seeded with the layout's bind
    /// pose.  Return it with [`release`](Self::release) when done.
    pub fn acquire(&mut self, layout: &PoseLayout) -> Pose {
        let mut pose = self.pool.pop().unwrap_or_default();
        layout.seed(&mut pose);
        pose
    }

    /// Returns a pose buffer to the pool, keeping its allocation for reuse.
    pub fn release(&mut self, pose: Pose) {
        self.pool.push(pose);
    }

    pub fn push(&mut self, evaluated: EvaluatedPose) {
        self.stack.push(evaluated);
    }

    pub fn pop(&mut self) -> Option<EvaluatedPose> {
        self.stack.pop()
    }

    pub fn stack_len(&self) -> usize {
        self.stack.len()
    }
}

pub struct AnimationGraphContext<'a> {
    pub(crate) animation_clips: &'a AssetStore<AnimationClip>,
    pub(crate) animation_graphs: &'a AssetStore<AnimationGraph>,
}

impl<'a> AnimationGraphContext<'a> {
    pub fn animation_clips(&self) -> &AssetStore<AnimationClip> {
        self.animation_clips
    }

    pub fn animation_graphs(&self) -> &AssetStore<AnimationGraph> {
        self.animation_graphs
    }
}
