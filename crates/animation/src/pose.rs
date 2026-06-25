
pub struct Pose {}

impl Pose {
    pub fn identity() -> Pose
    {
        todo!()
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