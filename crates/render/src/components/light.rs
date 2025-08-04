use bytemuck::Pod;
use ecs::{component::Component, resource::Resource};
use wgpu::{util::DeviceExt, BindGroupDescriptor, Buffer};

use crate::layouts::LightLayouts;

const MAX_LIGHTS: usize = 256;

#[derive(Component)]
pub enum Light {
    Point,
    Spot,
    Directional,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable)]
pub struct LightUniform {
    color: [f32; 4],
}

impl LightUniform {
    pub fn zeroed() -> Self {
        Self { color: [0.0; 4] }
    }
}

unsafe impl Pod for LightUniform {}

#[derive(Component)]
pub struct RenderLight {}

#[derive(Resource)]
pub(crate) struct RenderLights {
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) buffer: Buffer,
}

impl RenderLights {
    pub fn new(device: &wgpu::Device, layouts: &LightLayouts) -> Self {
        let lights = [LightUniform::zeroed(); MAX_LIGHTS];

        let lights_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lights_buffer"),
            contents: bytemuck::cast_slice(&lights),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let lights_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("lights_bind_group"),
            layout: &layouts.lights_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lights_buffer.as_entire_binding(),
            }],
        });

        Self {
            bind_group: lights_bind_group,
            buffer: lights_buffer,
        }
    }
}

pub(crate) fn update_lights_buffer() {}
