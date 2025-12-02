pub use essential_macros::Blendable;
use glam::{Quat, Vec3, Vec3A};

pub trait Blendable {
    fn lerp(self, other: Self, t: f32) -> Self;
}

impl Blendable for Vec3 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self.lerp(other, t)
    }
}

impl Blendable for Vec3A {
    fn lerp(self, other: Self, t: f32) -> Self {
        self.lerp(other, t)
    }
}

impl Blendable for Quat {
    fn lerp(self, other: Self, t: f32) -> Self {
        self.slerp(other, t)
    }
}