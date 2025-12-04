pub use essential_macros::Blendable;
use glam::{Quat, Vec3, Vec3A};

pub trait Blendable {
    fn interpolate(self, other: Self, t: f32) -> Self;
}

impl Blendable for Vec3 {
    fn interpolate(self, other: Self, t: f32) -> Self {
        self.lerp(other, t)
    }
}

impl Blendable for Vec3A {
    fn interpolate(self, other: Self, t: f32) -> Self {
        self.lerp(other, t)
    }
}

impl Blendable for Quat {
    fn interpolate(self, other: Self, t: f32) -> Self {
        self.slerp(other, t)
    }
}