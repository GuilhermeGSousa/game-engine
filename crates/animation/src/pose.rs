use essential::blend::Blendable;
use glam::{Quat, Vec3};

pub struct JointPose {
    pub translation: Vec3,
    pub rotation: Quat,
    // Uniform scaling? No scaling at all?
    pub scale: Vec3,
}

pub struct Pose(Box<[JointPose]>);

impl Pose {
    pub fn identity() -> Pose {
        todo!()
    }

    pub fn get_joint_pose(&mut self, bone_index: usize) -> Option<&JointPose> {
        self.0.get(bone_index)
    }

    pub fn get_joint_pose_mut(&mut self, bone_index: usize) -> Option<&mut JointPose> {
        self.0.get_mut(bone_index)
    }

    pub fn blend(&mut self, other: &Pose, weight: f32) {
        for (joint, other_joint) in self.0.iter_mut().zip(other.0.iter()) {
            joint.translation = joint.translation.lerp(other_joint.translation, weight);
            joint.rotation = joint.rotation.slerp(other_joint.rotation, weight);
            joint.scale = joint.scale.lerp(other_joint.scale, weight);
        }
    }
}

impl Blendable for Pose {
    fn interpolate(from: Self, to: Self, t: f32) -> Self {
        todo!()
    }
}

pub struct EvaluatedPose {
    pub pose: Pose,
    pub weight: f32,
}

#[derive(Default)]
pub struct PosePool {
    free_poses: Vec<Pose>,
}

impl PosePool {
    pub(crate) fn acquire(&mut self) -> Pose {
        self.free_poses.pop().unwrap_or(Pose::identity())
    }

    pub(crate) fn release(&mut self, pose: Pose) {
        self.free_poses.push(pose);
    }
}
