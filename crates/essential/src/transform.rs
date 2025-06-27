use bytemuck::Pod;
use ecs::component::Component;
use glam::{Mat4, Quat, Vec3};

#[derive(Component, Clone)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn compute_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    pub fn to_raw(&self) -> TransformRaw {
        TransformRaw {
            matrix: self.compute_matrix().to_cols_array_2d(),
        }
    }

    pub fn from_translation_rotation(translation: Vec3, rotation: Quat) -> Self {
        Self {
            translation: translation,
            rotation: rotation,
            scale: Vec3::ONE,
        }
    }

    pub fn look_to(&mut self, direction: Vec3, up: Vec3) {
        self.rotation = Quat::look_to_lh(direction, up);
    }

    pub fn local_x(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    pub fn local_y(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    pub fn local_z(&self) -> Vec3 {
        self.rotation * Vec3::Z
    }

    pub fn up(&self) -> Vec3 {
        self.local_y()
    }

    pub fn down(&self) -> Vec3 {
        -self.up()
    }

    pub fn forward(&self) -> Vec3 {
        -self.local_z()
    }

    pub fn backward(&self) -> Vec3 {
        -self.forward()
    }

    pub fn right(&self) -> Vec3 {
        self.local_x()
    }

    pub fn left(&self) -> Vec3 {
        -self.right()
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable)]
pub struct TransformRaw {
    matrix: [[f32; 4]; 4],
}

unsafe impl Pod for TransformRaw {}
