use color::{Color, LinearRgba};
use derive_more::Deref;
use ecs::{
    command::CommandQueue,
    component::Component,
    entity::Entity,
    query::{query_filter::Added, Query},
    resource::{Res, Resource},
    Changed,
};

use encase::{ShaderSize, ShaderType, UniformBuffer};
use essential::transform::GlobalTransform;
use glam::Vec3;
use wgpu::{util::DeviceExt, BindGroupDescriptor, Buffer};

use crate::{components::render_entity::RenderEntity, layouts::LightLayout, queue::RenderQueue};

const MAX_LIGHTS: usize = 128;

#[derive(Component)]
pub struct Light {
    pub color: Color,
    pub intensity: f32,
    pub shadowmaps_enabled: bool,
    pub light_type: LightType,
}

impl Light {
    pub fn point_light() -> Self {
        Self {
            color: Color::WHITE,
            intensity: 1.0,
            shadowmaps_enabled: false,
            light_type: LightType::Point,
        }
    }

    pub fn spot_light(cone_angle: f32) -> Self {
        Self {
            color: Color::WHITE,
            intensity: 1.0,
            shadowmaps_enabled: false,
            light_type: LightType::Spot { cone_angle },
        }
    }

    pub fn directional_light() -> Self {
        Self {
            color: Color::WHITE,
            intensity: 1.0,
            shadowmaps_enabled: false,
            light_type: LightType::Directional,
        }
    }

    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_shadows(mut self) -> Self {
        self.shadowmaps_enabled = true;
        self
    }
}

pub enum LightType {
    Point,
    Spot { cone_angle: f32 },
    Directional,
}

impl LightType {
    pub fn index(&self) -> u32 {
        match *self {
            LightType::Point => 0,
            LightType::Spot { .. } => 1,
            LightType::Directional => 2,
        }
    }
}

#[derive(ShaderType)]
pub(crate) struct LightsUniform {
    pub(crate) lights: [RenderLight; MAX_LIGHTS],
    pub(crate) light_count: i32,
}

#[derive(Component, Clone, Copy, Deref)]
pub struct RenderLightSlot(u32);

impl RenderLightSlot {
    pub(crate) fn update_slot(&mut self, new_slot: u32) {
        self.0 = new_slot;
    }
}

#[derive(ShaderType, Clone, Copy)]
pub struct RenderLight {
    pub(crate) translation: Vec3,
    pub(crate) intensity: f32,
    pub(crate) color: LinearRgba,
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
            color: LinearRgba::TRANSPARENT,
            direction: Vec3::ZERO,
            light_type: 0,
            cos_cone_angle: 0.0,
        }
    }
}

impl Component for RenderLight {
    fn name() -> &'static str {
        std::any::type_name::<RenderLight>()
    }

    fn on_add() -> Option<ecs::component::ComponentLifecycleCallback> {
        Some(|mut world, context| {
            let slot = if let Some(lights) = world.get_resource_mut::<RenderLights>() {
                lights.push_light(context.entity);
                RenderLightSlot(lights.len() as u32 - 1)
            } else {
                return;
            };

            world.insert_component(slot, context.entity, false);

            if let (Some(lights), Some(queue)) = (
                world.get_resource::<RenderLights>(),
                world.get_resource::<RenderQueue>(),
            ) {
                lights.write_count(&queue);
            }
        })
    }

    fn on_remove() -> Option<ecs::component::ComponentLifecycleCallback> {
        Some(|mut world, context| {
            let Some(&slot) = world.get_component_for_entity::<RenderLightSlot>(context.entity)
            else {
                return;
            };

            let moved_entity = if let Some(lights) = world.get_resource_mut::<RenderLights>() {
                lights.swap_remove_light(&slot)
            } else {
                return;
            };

            world.remove_component::<RenderLightSlot>(context.entity, false);

            if let Some(moved_entity) = moved_entity {
                if let Some(moved_slot) =
                    world.get_component_for_entity_mut::<RenderLightSlot>(moved_entity)
                {
                    moved_slot.update_slot(*slot);
                }

                let moved_light = world
                    .get_component_for_entity::<RenderLight>(moved_entity)
                    .copied();

                if let (Some(moved_light), Some(lights), Some(queue)) = (
                    moved_light,
                    world.get_resource::<RenderLights>(),
                    world.get_resource::<RenderQueue>(),
                ) {
                    lights.write_buffer(&queue, &moved_light, slot);
                }
            }

            if let (Some(lights), Some(queue)) = (
                world.get_resource::<RenderLights>(),
                world.get_resource::<RenderQueue>(),
            ) {
                lights.write_count(&queue);
            }
        })
    }
}

#[derive(Resource)]
pub(crate) struct RenderLights {
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) buffer: Buffer,
    pub(crate) slots: Vec<Entity>,
}

impl RenderLights {
    pub(crate) fn new(device: &wgpu::Device, layout: &LightLayout) -> Self {
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
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lights_buffer.as_entire_binding(),
            }],
        });

        Self {
            bind_group: lights_bind_group,
            buffer: lights_buffer,
            slots: Vec::new(),
        }
    }

    pub(crate) fn write_buffer(
        &self,
        queue: &wgpu::Queue,
        light: &RenderLight,
        offset: RenderLightSlot,
    ) {
        let slot_offset = light.size().get() * *offset as u64;

        let mut buffer = UniformBuffer::new(Vec::new());
        buffer.write(light).unwrap();
        queue.write_buffer(&self.buffer, slot_offset, &buffer.into_inner());
    }

    pub(crate) fn write_count(&self, queue: &wgpu::Queue) {
        let count_offset = RenderLight::SHADER_SIZE.get() * MAX_LIGHTS as u64;
        let count = self.slots.len() as i32;

        let mut buffer = UniformBuffer::new(Vec::new());
        buffer.write(&count).unwrap();
        queue.write_buffer(&self.buffer, count_offset, &buffer.into_inner());
    }

    pub(crate) fn push_light(&mut self, light: Entity) {
        self.slots.push(light);
    }

    pub(crate) fn swap_remove_light(&mut self, slot: &RenderLightSlot) -> Option<Entity> {
        let index = **slot as usize;
        self.slots.swap_remove(index);
        self.slots.get(index).copied()
    }

    pub(crate) fn len(&self) -> usize {
        self.slots.len()
    }
}

pub(crate) fn update_changed_lights(
    lights: Query<(&RenderLight, &RenderLightSlot), Changed<RenderLight>>,
    lights_buffer: Res<RenderLights>,
    queue: Res<RenderQueue>,
) {
    for (light, slot) in lights.iter() {
        lights_buffer.write_buffer(&queue, light, *slot);
    }
}

pub(crate) fn light_added(
    lights: Query<(Entity, &Light, &GlobalTransform, Option<&RenderEntity>), Added<Light>>,
    mut cmd: CommandQueue,
) {
    for (entity, light, light_transform, render_entity) in lights.iter() {
        let local_z = light_transform.rotation() * Vec3::Z;
        let render_light = RenderLight {
            translation: light_transform.translation(),
            color: light.color.to_linear(),
            intensity: light.intensity,
            direction: -local_z,
            light_type: light.light_type.index(),
            cos_cone_angle: match &light.light_type {
                LightType::Spot { cone_angle } => f32::cos(*cone_angle),
                _ => 0.0,
            },
        };
        match render_entity {
            None => {
                let new_render_entity = cmd.spawn(render_light).entity();
                cmd.insert(RenderEntity::new(new_render_entity), entity);
            }
            Some(render_entity) => {
                cmd.insert(render_light, **render_entity);
            }
        }
    }
}

pub(crate) fn light_changed(
    lights: Query<
        (&Light, &GlobalTransform, &RenderEntity),
        ecs::Or<(Changed<Light>, Changed<GlobalTransform>)>,
    >,
    render_lights: Query<&mut RenderLight>,
) {
    for (light, transform, render_entity) in lights.iter() {
        if let Some(mut render_light) = render_lights.get_entity(**render_entity) {
            let local_z = transform.rotation() * Vec3::Z;
            render_light.direction = -local_z;
            render_light.color = light.color.to_linear();
            render_light.translation = transform.translation();
            render_light.intensity = light.intensity;
            render_light.light_type = light.light_type.index();
            render_light.cos_cone_angle = match &light.light_type {
                LightType::Spot { cone_angle } => f32::cos(*cone_angle),
                _ => 0.0,
            };
        }
    }
}
