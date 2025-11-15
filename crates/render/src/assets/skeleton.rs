use essential::assets::Asset;
use glam::Mat4;

#[derive(Asset)]
pub struct Skeleton {
    pub(crate) inverse_bindposes: Box<[Mat4]>,
}

impl From<Vec<Mat4>> for Skeleton {
    fn from(value: Vec<Mat4>) -> Self {
        Self {
            inverse_bindposes: value.into_boxed_slice(),
        }
    }
}
