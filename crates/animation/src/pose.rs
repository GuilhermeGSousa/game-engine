use std::collections::HashMap;

use essential::{blend::Blendable, transform::Transform};
use uuid::Uuid;

/// A full-skeleton pose: one local [`Transform`] per bone, indexed by the bone's position in
/// the owning skeleton (see [`mesh::skeleton::SkeletonComponent::bones`]).  This is the unit
/// the animation graph passes between nodes.
#[derive(Default, Clone)]
pub struct Pose {
    pub transforms: Vec<Transform>,
}

impl Pose {
    /// Blends `other` into `self` element-wise, in place: `self[i] = lerp(self[i], other[i], t)`.
    ///
    /// Both poses are expected to share a layout (and therefore length).
    pub fn interpolate_into(&mut self, other: &Pose, t: f32) {
        for (dst, src) in self.transforms.iter_mut().zip(&other.transforms) {
            *dst = Transform::interpolate(dst.clone(), src.clone(), t);
        }
    }
}

/// Per-player, animation-specific description of a skeleton's bones, indexed in
/// [`SkeletonComponent::bones`](mesh::skeleton::SkeletonComponent::bones) order.
///
/// It deliberately holds *only* the data the skeleton component does not: the animation
/// channel id of each bone (for clip sampling and IK lookups) and the bind/rest pose used to
/// seed evaluation.  The bone entities themselves are read live from the
/// `SkeletonComponent`, so they are not duplicated here.
#[derive(Default)]
pub struct PoseLayout {
    target_ids: Vec<Option<Uuid>>,
    index_of_target: HashMap<Uuid, usize>,
    bind_pose: Vec<Transform>,
}

impl PoseLayout {
    /// Builds a layout from parallel per-bone data (same order as the skeleton's bones).
    pub fn new(target_ids: Vec<Option<Uuid>>, bind_pose: Vec<Transform>) -> Self {
        let mut index_of_target = HashMap::new();
        for (index, target_id) in target_ids.iter().enumerate() {
            if let Some(id) = target_id {
                index_of_target.insert(*id, index);
            }
        }
        Self {
            target_ids,
            index_of_target,
            bind_pose,
        }
    }

    pub fn len(&self) -> usize {
        self.bind_pose.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bind_pose.is_empty()
    }

    /// Per-bone animation channel ids; `None` for un-animated bones.
    pub fn target_ids(&self) -> &[Option<Uuid>] {
        &self.target_ids
    }

    /// Resolves the bone index for an animated target, if present in this skeleton.
    pub fn index_of(&self, target_id: &Uuid) -> Option<usize> {
        self.index_of_target.get(target_id).copied()
    }

    /// Resizes `pose` to this layout and resets every bone to its bind transform.
    pub fn seed(&self, pose: &mut Pose) {
        pose.transforms.clear();
        pose.transforms.extend_from_slice(&self.bind_pose);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::AnimationGraphEvaluator;
    use glam::Vec3;

    fn translation(x: f32) -> Transform {
        Transform {
            translation: Vec3::new(x, 0.0, 0.0),
            ..Transform::IDENTITY
        }
    }

    fn two_bone_layout() -> (PoseLayout, Uuid) {
        let target_id = Uuid::new_v4();
        // bone 0 is animated, bone 1 is not.
        let target_ids = vec![Some(target_id), None];
        let bind_pose = vec![translation(1.0), translation(2.0)];
        (PoseLayout::new(target_ids, bind_pose), target_id)
    }

    #[test]
    fn layout_resolves_targets_and_seeds_bind_pose() {
        let (layout, target_id) = two_bone_layout();

        assert_eq!(layout.len(), 2);
        assert_eq!(layout.index_of(&target_id), Some(0));
        assert_eq!(layout.index_of(&Uuid::new_v4()), None);

        let mut pose = Pose::default();
        layout.seed(&mut pose);

        assert_eq!(pose.transforms.len(), 2);
        assert_eq!(pose.transforms[0].translation, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(pose.transforms[1].translation, Vec3::new(2.0, 0.0, 0.0));
    }

    #[test]
    fn evaluator_acquire_reseeds_recycled_buffers() {
        let (layout, _) = two_bone_layout();
        let mut evaluator = AnimationGraphEvaluator::new();

        let mut pose = evaluator.acquire(&layout);
        assert_eq!(pose.transforms.len(), 2);
        // Mutate then return the buffer to the pool.
        pose.transforms[0] = translation(99.0);
        evaluator.release(pose);

        // The recycled buffer must come back reset to the bind pose.
        let pose = evaluator.acquire(&layout);
        assert_eq!(pose.transforms.len(), 2);
        assert_eq!(pose.transforms[0].translation, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn interpolate_into_blends_elementwise() {
        let mut a = Pose {
            transforms: vec![translation(0.0)],
        };
        let b = Pose {
            transforms: vec![translation(10.0)],
        };

        a.interpolate_into(&b, 0.5);
        assert!((a.transforms[0].translation.x - 5.0).abs() < 1e-5);

        a.interpolate_into(&b, 1.0);
        assert!((a.transforms[0].translation.x - 10.0).abs() < 1e-5);
    }
}
