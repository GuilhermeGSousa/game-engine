use cgmath::{Matrix4, SquareMatrix as _, Transform as cTransform};
use ecs::component::Component;

#[derive(Component)]
pub struct Transform {
    matrix: Matrix4<f32>,
}

impl Transform {
    pub fn from_matrix(matrix: Matrix4<f32>) -> Self {
        Self { matrix: matrix }
    }

    fn inverse(&self) -> Option<Self> {
        let inverse = self.matrix.inverse_transform();

        match inverse {
            Some(inverse) => Some(Self { matrix: inverse }),
            None => None,
        }
    }

    fn identity() -> Self {
        Self {
            matrix: Matrix4::<f32>::identity(),
        }
    }
}

impl std::ops::Mul<Transform> for Transform {
    type Output = Transform;

    fn mul(self, rhs: Self) -> Self {
        Self {
            matrix: self.matrix * rhs.matrix,
        }
    }
}
