use bytemuck::Pod;
use ecs::component::{Component, ComponentLifecycleCallback};
use glam::{Mat3, Mat4, Quat, Vec3};

pub mod systems;

#[derive(Clone)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Component for Transform {
    fn name() -> String {
        String::from("Transform")
    }

    fn on_add() -> Option<ComponentLifecycleCallback> {
        Some(|mut world, context| {
            let global_transform = GlobalTranform::new(
                world
                    .get_component_for_entity::<Transform>(context.entity)
                    .unwrap()
                    .compute_matrix(),
            );

            world.insert_component(global_transform, context.entity);
        })
    }
}

impl Transform {
    pub fn compute_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    pub fn compute_rotation_matrix(&self) -> Mat3 {
        Mat3::from_quat(self.rotation)
    }

    pub fn to_raw(&self) -> TransformRaw {
        TransformRaw {
            matrix: self.compute_matrix().to_cols_array_2d(),
            rotation_matrix: self.compute_rotation_matrix().to_cols_array_2d(),
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
        self.rotation = Quat::look_to_rh(direction, up);
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
    rotation_matrix: [[f32; 3]; 3],
}

unsafe impl Pod for TransformRaw {}

#[derive(Component)]
pub struct GlobalTranform(Mat4);

impl GlobalTranform {
    pub fn new(transform: Mat4) -> Self {
        Self(transform)
    }
}
