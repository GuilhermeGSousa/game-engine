use bytemuck::Pod;
use ecs::{
    component::Component,
    query::Query,
    resource::{Res, Resource},
};
use essential::transform::Transform;
use glam::Vec3;
use wgpu::{util::DeviceExt, BindGroupDescriptor, Buffer};

use crate::{layouts::LightLayouts, resources::RenderContext};

const MAX_LIGHTS: usize = 256;

#[derive(Component)]
pub enum Light {
    Point,
    Spot,
    Directional,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable)]
pub(crate) struct LightUniform {
    color: [f32; 4],
    position: [f32; 3],
    _padding_position: f32,
}

impl LightUniform {
    pub fn zeroed() -> Self {
        Self {
            color: [0.0, 0.0, 1.0, 1.0],
            position: [0.0; 3],
            _padding_position: 0.0,
        }
    }
}

unsafe impl Pod for LightUniform {}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable)]
pub(crate) struct LightsUniform {
    pub(crate) lights: [LightUniform; MAX_LIGHTS],
    pub(crate) light_count: i32,
    pub(crate) _pad_light_count: [i32; 3],
}

unsafe impl Pod for LightsUniform {}

#[derive(Component)]
pub struct RenderLight {
    pub(crate) translation: Vec3,
}

impl RenderLight {
    pub(crate) fn to_uniform(&self) -> LightUniform {
        LightUniform {
            color: [1.0, 0.0, 1.0, 1.0],
            position: self.translation.into(),
            _padding_position: 0.0,
        }
    }
}

#[derive(Resource)]
pub(crate) struct RenderLights {
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) buffer: Buffer,
}

impl RenderLights {
    pub fn new(device: &wgpu::Device, layouts: &LightLayouts) -> Self {
        let lights = LightsUniform {
            lights: [LightUniform::zeroed(); MAX_LIGHTS],
            light_count: 0,
            _pad_light_count: [0; 3],
        };

        let lights_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lights_buffer"),
            contents: bytemuck::cast_slice(&[lights]),
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

    pub fn write_buffer(&self, queue: &wgpu::Queue, uniform: LightsUniform) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[uniform]));
    }
}

pub(crate) fn update_lights_buffer(
    lights: Query<&RenderLight>,
    lights_buffer: Res<RenderLights>,
    context: Res<RenderContext>,
) {
    let mut light_array = [LightUniform::zeroed(); MAX_LIGHTS];
    let mut current_index = 0;
    for light in lights.iter() {
        light_array[current_index] = light.to_uniform();
        current_index += 1;
    }

    lights_buffer.write_buffer(
        &context.queue,
        LightsUniform {
            lights: light_array,
            light_count: current_index as i32,
            _pad_light_count: [0; 3],
        },
    );
}
