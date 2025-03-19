use cgmath::{Matrix4, Transform as cTransform};
use ecs::component::Component;

#[derive(Component)]
pub struct Transform {
    matrix: Matrix4<f32>,
}

impl Transform {
    fn new() -> Self {
        todo!()
    }

    fn inverse(&self) -> Option<Self> {
        let inverse = self.matrix.inverse_transform();

        match inverse {
            Some(inverse) => Some(Self { matrix: inverse }),
            None => None,
        }
    }
}
