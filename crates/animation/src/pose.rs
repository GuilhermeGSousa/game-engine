use glam::{Quat, Vec3};


pub struct JointPose
{
    pub translation: Vec3,
    pub rotation: Quat,
    // Uniform scaling? No scaling at all?
    pub scale: Vec3,
}

pub struct Pose(Box<[JointPose]>);

impl Pose {
    pub fn identity() -> Pose
    {
        todo!()
    }

    pub fn get_joint_pose(&mut self, bone_index: usize) -> Option<&JointPose>
    {
        self.0.get(bone_index)
    }

    pub fn get_joint_pose_mut(&mut self, bone_index: usize) -> Option<&mut JointPose>
    {
        self.0.get_mut(bone_index)
    }
}

pub struct EvaluatedPose {
    pub pose: Pose,
    pub weight: f32,
}

#[derive(Default)]
pub(crate) struct PosePool 
{
    free_poses: Vec<Pose>
}

impl PosePool {
    pub(crate) fn acquire(&mut self) -> Pose
    {
        self.free_poses.pop().unwrap_or(Pose::identity())
    }

    pub(crate) fn release(&mut self, pose: Pose)
    {
        self.free_poses.push(pose);
    }
}