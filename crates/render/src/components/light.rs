use bytemuck::Pod;
use ecs::{
    component::Component,
    query::Query,
    resource::{Res, Resource},
};

use glam::{Vec3, Vec4};
use wgpu::{util::DeviceExt, BindGroupDescriptor, Buffer};

use crate::{layouts::LightLayouts, resources::RenderContext};

const MAX_LIGHTS: usize = 256;

#[derive(Component)]
pub struct Light {
    pub color: Vec4,
    pub intensity: f32,
    pub light_type: LighType,
}

#[derive(Clone, Copy)]
pub enum LighType {
    Point = 1,
    Spot = 2,
    Directional = 3,
}

pub struct SpotLight {
    pub cone_angle: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub(crate) struct LightUniform {
    position: [f32; 3],
    _position_padding: f32,
    color: [f32; 4],
    intensity: f32,
    direction: [f32; 3],
    light_type: u32,
    _padding: [u32; 3],
}

impl LightUniform {
    pub fn zeroed() -> Self {
        Self {
            position: [0.0; 3],
            _position_padding: 0.0,
            color: [0.0, 0.0, 1.0, 1.0],
            intensity: 0.0,
            direction: [0.0; 3],
            light_type: 0,
            _padding: [0; 3],
        }
    }
}

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
    pub(crate) intensity: f32,
    pub(crate) color: Vec4,
    pub(crate) direction: Vec3,
    pub(crate) light_type: u32,
}

impl RenderLight {
    pub(crate) fn to_uniform(&self) -> LightUniform {
        let mut uniform = LightUniform::zeroed();
        uniform.direction = self.direction.into();
        uniform.position = self.translation.into();
        uniform.color = self.color.into();
        uniform.intensity = self.intensity;
        // uniform.light_type = self.light_type;

        let bytes: &[u8] = bytemuck::bytes_of(&uniform);
        println!("Byte representation: {:?}", bytes);
        uniform
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
