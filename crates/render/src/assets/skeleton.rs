use essential::assets::Asset;
use glam::Mat4;


pub struct Skeleton
{
    pub(crate) inverse_bindposes: Box<[Mat4]>
}

impl Asset for Skeleton {
    
}