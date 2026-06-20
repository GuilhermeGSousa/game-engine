use std::collections::HashMap;

use ecs::{component::Component, entity::Entity};
use essential::{blend::Blendable, transform::Transform};
use uuid::Uuid;

/// A single bone in a player's skeleton, in skeleton order.
///
/// Built once at setup and stored inside a [`PoseLayout`].  It carries everything the
/// animation graph needs to evaluate and apply a full pose: the bone entity to write the
/// result back to, the animation-channel key (for bones that are actually animated), and
/// the bind/rest local transform used to seed un-animated bones.
#[derive(Clone)]
pub struct PoseBone {
    /// Entity whose `Transform` the evaluated pose is written back to.
    pub entity: Entity,
    /// Animation-clip channel key.  `Some` only for bones that appear in a clip; `None`
    /// bones keep their bind pose.
    pub target_id: Option<Uuid>,
    /// Rest/bind local transform, used as the seed value for this bone every frame.
    pub bind_local: Transform,
    /// `true` for the skeleton's root-motion bone (see `apply_poses`).
    pub is_root: bool,
}

/// A full-skeleton pose: one local [`Transform`] per bone, indexed by the bone's position
/// in the owning [`PoseLayout`].  This is the unit the animation graph passes between nodes.
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

/// Stable, per-player description of the skeleton the graph animates.
///
/// Owns the ordered bone list plus fast lookups: target-id → bone index (for clip sampling)
/// and a cached bind pose (for seeding pose buffers cheaply).
#[derive(Default)]
pub struct PoseLayout {
    bones: Vec<PoseBone>,
    index_of_target: HashMap<Uuid, usize>,
    bind_pose: Vec<Transform>,
}

impl PoseLayout {
    pub fn from_bones(bones: Vec<PoseBone>) -> Self {
        let mut index_of_target = HashMap::new();
        let mut bind_pose = Vec::with_capacity(bones.len());
        for (index, bone) in bones.iter().enumerate() {
            if let Some(id) = bone.target_id {
                index_of_target.insert(id, index);
            }
            bind_pose.push(bone.bind_local.clone());
        }
        Self {
            bones,
            index_of_target,
            bind_pose,
        }
    }

    pub fn len(&self) -> usize {
        self.bones.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bones.is_empty()
    }

    pub fn bones(&self) -> &[PoseBone] {
        &self.bones
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

/// Hand-off component carrying the ordered bone list a player drives.
///
/// Inserted by the glTF loader on the player entity and consumed once (via the
/// `build_pose_layouts` system) into the player's [`PoseLayout`].  This keeps the animation
/// crate free of a dependency on the render crate, where the skeleton GPU data lives.
#[derive(Component)]
pub struct AnimationSkeleton {
    pub bones: Vec<PoseBone>,
}

impl AnimationSkeleton {
    pub fn new(bones: Vec<PoseBone>) -> Self {
        Self { bones }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::AnimationGraphEvaluator;
    use ecs::world::World;
    use glam::Vec3;

    fn translation(x: f32) -> Transform {
        Transform {
            translation: Vec3::new(x, 0.0, 0.0),
            ..Transform::IDENTITY
        }
    }

    fn two_bone_layout(world: &mut World) -> (PoseLayout, Uuid) {
        let animated = world.spawn(Transform::IDENTITY);
        let un_animated = world.spawn(Transform::IDENTITY);
        let target_id = Uuid::new_v4();

        let bones = vec![
            PoseBone {
                entity: animated,
                target_id: Some(target_id),
                bind_local: translation(1.0),
                is_root: false,
            },
            PoseBone {
                entity: un_animated,
                target_id: None,
                bind_local: translation(2.0),
                is_root: true,
            },
        ];

        (PoseLayout::from_bones(bones), target_id)
    }

    #[test]
    fn layout_resolves_targets_and_seeds_bind_pose() {
        let mut world = World::new();
        let (layout, target_id) = two_bone_layout(&mut world);

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
        let mut world = World::new();
        let (layout, _) = two_bone_layout(&mut world);
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
