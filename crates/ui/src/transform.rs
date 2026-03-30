use std::mem;

use derive_more::Deref;
use ecs::component::Component;
use glam::Affine2;
use render::assets::vertex::VertexBufferLayout;
use wgpu::VertexFormat;

#[derive(Default)]
pub enum UIValue {
    #[default]
    Auto,
    Px(f32),
    Percent(f32),
}

#[derive(Default)]
pub struct UIValue2 {
    pub x: UIValue,
    pub y: UIValue,
}

#[derive(Component, Default)]
pub struct UITransform {
    pub translation: UIValue2,
}

#[derive(Component, Deref)]
pub struct UIGlobalTransform(glam::Affine2);

impl Default for UIGlobalTransform {
    fn default() -> Self {
        Self(Affine2::IDENTITY)
    }
}

impl VertexBufferLayout for UIGlobalTransform {
    fn describe() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<UIGlobalTransform>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Float32x2,
                },
            ],
        }
    }
}
