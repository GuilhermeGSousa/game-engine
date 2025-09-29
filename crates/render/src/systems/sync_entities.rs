use ecs::{command::CommandQueue, query::query_filter::Added, query::Query, resource::Res};
use encase::UniformBuffer;
use essential::transform::GlobalTranform;
use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::{
    components::{
        camera::{Camera, CameraUniform, RenderCamera},
        light::{LighType, Light, RenderLight},
        render_entity::RenderEntity,
    },
    device::RenderDevice,
    layouts::CameraLayouts,
    queue::RenderQueue,
    render_asset::render_texture::RenderTexture,
    resources::RenderContext,
};

pub(crate) fn camera_added(
    cameras: Query<(Entity, &Camera, &GlobalTranform, Option<&RenderEntity>), Added<(Camera,)>>,
    mut cmd: CommandQueue,
    device: Res<RenderDevice>,
    context: Res<RenderContext>,
    camera_layouts: Res<CameraLayouts>,
) {
    for (entity, camera, transform, render_entity) in cameras.iter() {
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(camera, transform);

        let mut buffer = UniformBuffer::new(Vec::new());
        buffer.write(&camera_uniform).unwrap();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: &buffer.into_inner(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_layouts.camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let depth_texture =
            RenderTexture::create_depth_texture(&device, &context.surface_config, "depth_texture");

        let render_cam = RenderCamera {
            camera_bind_group: camera_bind_group,
            camera_uniform: camera_uniform,
            camera_buffer: camera_buffer,
            depth_texture: depth_texture,
            clear_color: camera.clear_color,
        };

        match render_entity {
            None => {
                let new_render_entity = cmd.spawn(render_cam);
                cmd.insert(RenderEntity::new(new_render_entity), entity);
            }
            Some(render_entity) => {
                cmd.insert(render_cam, **render_entity);
            }
        }
    }
}

pub(crate) fn camera_changed(
    cameras: Query<(&Camera, &GlobalTranform, &RenderEntity)>,
    render_cameras: Query<(&mut RenderCamera,)>,
    queue: Res<RenderQueue>,
) {
    for (camera, transform, render_entity) in cameras.iter() {
        if let Some((mut render_camera,)) = render_cameras.get_entity(**render_entity) {
            render_camera
                .camera_uniform
                .update_view_proj(camera, transform);

            let mut buffer = UniformBuffer::new(Vec::new());
            buffer.write(&render_camera.camera_uniform).unwrap();

                    queue.write_buffer(&render_camera.camera_buffer, 0, &buffer.into_inner());
                }
            }
            _ => {}
        }
    }
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
