use ecs::{
    command::CommandQueue, component::Component, entity::Entity, query::{query_filter::Added, Query}, resource::{Res, Resource}
};

use encase::{ShaderType, UniformBuffer};
use essential::transform::GlobalTranform;
use glam::{Vec3, Vec4};
use wgpu::{util::DeviceExt, BindGroupDescriptor, Buffer};

use crate::{components::render_entity::RenderEntity, layouts::LightLayouts, queue::RenderQueue};

const MAX_LIGHTS: usize = 128;

#[derive(Component)]
pub struct Light {
    pub color: Vec4,
    pub intensity: f32,
    pub light_type: LighType,
}

pub struct SpotLight {
    pub cone_angle: f32,
}

pub enum LighType {
    Point,
    Spot(SpotLight),
    Directional,
}

impl LighType {
    pub fn index(&self) -> u32 {
        match *self {
            LighType::Point => 0,
            LighType::Spot(_) => 1,
            LighType::Directional => 1,
        }
    }
}

#[derive(ShaderType)]
pub(crate) struct LightsUniform {
    pub(crate) lights: [RenderLight; MAX_LIGHTS],
    pub(crate) light_count: i32,
}

#[derive(Component, ShaderType, Clone, Copy)]
pub struct RenderLight {
    pub(crate) translation: Vec3,
    pub(crate) intensity: f32,
    pub(crate) color: Vec4,
    pub(crate) direction: Vec3,
    pub(crate) light_type: u32,

    // Spotlight
    pub(crate) cos_cone_angle: f32,
}

impl RenderLight {
    pub(crate) fn zeroed() -> Self {
        Self {
            translation: Vec3::ZERO,
            intensity: 0.0,
            color: Vec4::ZERO,
            direction: Vec3::ZERO,
            light_type: 0,
            cos_cone_angle: 0.0,
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
            lights: [RenderLight::zeroed(); MAX_LIGHTS],
            light_count: 0,
        };

        let mut buffer = UniformBuffer::new(Vec::new());
        buffer.write(&lights).unwrap();

        let lights_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lights_buffer"),
            contents: &buffer.into_inner(),
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
        let mut buffer = UniformBuffer::new(Vec::new());
        buffer.write(&uniform).unwrap();
        queue.write_buffer(&self.buffer, 0, &buffer.into_inner());
    }
}

pub(crate) fn prepare_lights_buffer(
    lights: Query<&RenderLight>,
    lights_buffer: Res<RenderLights>,
    context: Res<RenderQueue>,
) {
    let mut light_array = [RenderLight::zeroed(); MAX_LIGHTS];
    let mut current_index = 0;
    for light in lights.iter() {
        light_array[current_index] = *light;
        current_index += 1;
    }

    lights_buffer.write_buffer(
        &context.queue,
        LightsUniform {
            lights: light_array,
            light_count: current_index as i32,
        },
    );
}


pub(crate) fn light_added(
    lights: Query<(Entity, &Light, &GlobalTranform, Option<&RenderEntity>), Added<Light>>,
    mut cmd: CommandQueue,
) {
    for (entity, light, light_transform, render_entity) in lights.iter() {
        let local_z = light_transform.rotation() * Vec3::Z;
        let render_light = RenderLight {
            translation: light_transform.translation(),
            color: light.color,
            intensity: light.intensity,
            direction: -local_z,
            light_type: light.light_type.index(),
            cos_cone_angle: match &light.light_type {
                LighType::Spot(spot_light) => f32::cos(spot_light.cone_angle),
                _ => 0.0,
            },
        };
        match render_entity {
            None => {
                let new_render_entity = cmd.spawn(render_light);
                cmd.insert(RenderEntity::new(new_render_entity), entity);
            }
            Some(render_entity) => {
                cmd.insert(render_light, **render_entity);
            }
        }
    }
}

pub(crate) fn light_changed(
    lights: Query<(&Light, &GlobalTranform, &RenderEntity)>,
    render_lights: Query<&mut RenderLight>,
) {
    for (light, transform, render_entity) in lights.iter() {
        if let Some(mut render_light) = render_lights.get_entity(**render_entity) {
            let local_z = transform.rotation() * Vec3::Z;
            render_light.direction = -local_z;
            render_light.color = light.color;
            render_light.translation = transform.translation();
            render_light.intensity = light.intensity;
            render_light.light_type = light.light_type.index();
            render_light.cos_cone_angle = match &light.light_type {
                LighType::Spot(spot_light) => f32::cos(spot_light.cone_angle),
                _ => 0.0,
            };
        }
    }
}
