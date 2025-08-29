use ecs::{
    command::CommandQueue,
    query::query_filter::{Added, Changed},
    query::Query,
    resource::Res,
};
use encase::UniformBuffer;
use essential::transform::Transform;
use wgpu::util::DeviceExt;

use crate::{
    components::{
        camera::{Camera, CameraUniform, RenderCamera},
        light::{LighType, Light, RenderLight},
        mesh_component::{MeshComponent, RenderMeshInstance},
        render_entity::RenderEntity,
    },
    device::RenderDevice,
    layouts::CameraLayouts,
    queue::RenderQueue,
    render_asset::render_texture::RenderTexture,
    resources::RenderContext,
};

pub(crate) fn camera_added(
    cameras: Query<(&Camera, &Transform, &mut RenderEntity), Added<(Camera,)>>,
    mut cmd: CommandQueue,
    device: Res<RenderDevice>,
    context: Res<RenderContext>,
    camera_layouts: Res<CameraLayouts>,
) {
    for (camera, transform, mut render_entity) in cameras.iter() {
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

        match **render_entity {
            RenderEntity::Uninitialized => {
                let new_entity = cmd.spawn(render_cam);
                render_entity.set_entity(new_entity);
            }
            RenderEntity::Initialized(entity) => {
                cmd.insert(render_cam, entity);
            }
        }
    }
}

pub(crate) fn camera_changed(
    cameras: Query<(&Camera, &Transform, &RenderEntity), Changed<(Transform,)>>,
    render_cameras: Query<(&mut RenderCamera,)>,
    queue: Res<RenderQueue>,
) {
    for (camera, transform, render_entity) in cameras.iter() {
        match render_entity {
            RenderEntity::Initialized(entity) => {
                if let Some((mut render_camera,)) = render_cameras.get_entity(*entity) {
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

pub(crate) fn mesh_added(
    meshes: Query<(&MeshComponent, &Transform, &mut RenderEntity), Added<(MeshComponent,)>>,
    mut cmd: CommandQueue,
    device: Res<RenderDevice>,
) {
    for (mesh, transform, mut render_entity) in meshes.iter() {
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&[transform.to_raw()]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let instance: RenderMeshInstance = RenderMeshInstance {
            render_asset_id: mesh.handle.id(),
            buffer: instance_buffer,
        };

        match **render_entity {
            RenderEntity::Uninitialized => {
                let new_entity = cmd.spawn(instance);
                render_entity.set_entity(new_entity);
            }
            RenderEntity::Initialized(entity) => {
                cmd.insert(instance, entity);
            }
        }
    }
}

pub(crate) fn mesh_changed(
    meshes: Query<(&MeshComponent, &Transform, &RenderEntity), Changed<(Transform,)>>,
    render_meshes: Query<(&mut RenderMeshInstance,)>,
    queue: Res<RenderQueue>,
) {
    for (_, transform, render_entity) in meshes.iter() {
        match render_entity {
            RenderEntity::Initialized(entity) => {
                if let Some((render_mesh,)) = render_meshes.get_entity(*entity) {
                    queue.write_buffer(
                        &render_mesh.buffer,
                        0,
                        bytemuck::cast_slice(&[transform.to_raw()]),
                    );
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn light_added(
    lights: Query<(&Light, &Transform, &mut RenderEntity), Added<Light>>,
    mut cmd: CommandQueue,
) {
    for (light, light_transform, mut render_entity) in lights.iter() {
        let render_light = RenderLight {
            translation: light_transform.translation,
            color: light.color,
            intensity: light.intensity,
            direction: light_transform.forward(),
            light_type: light.light_type.index(),
            cos_cone_angle: match &light.light_type {
                LighType::Spot(spot_light) => f32::cos(spot_light.cone_angle),
                _ => 0.0,
            },
        };
        match **render_entity {
            RenderEntity::Uninitialized => {
                let new_entity = cmd.spawn(render_light);
                render_entity.set_entity(new_entity);
            }
            RenderEntity::Initialized(entity) => {
                cmd.insert(render_light, entity);
            }
        }
    }
}

pub(crate) fn light_changed(
    lights: Query<(&Light, &Transform, &RenderEntity), Changed<(Transform,)>>,
    render_lights: Query<&mut RenderLight>,
) {
    for (light, transform, render_entity) in lights.iter() {
        match render_entity {
            RenderEntity::Uninitialized => {}
            RenderEntity::Initialized(entity) => {
                if let Some(mut render_light) = render_lights.get_entity(*entity) {
                    render_light.direction = transform.forward();
                    render_light.color = light.color;
                    render_light.translation = transform.translation;
                    render_light.intensity = light.intensity;
                    render_light.light_type = light.light_type.index();
                    render_light.cos_cone_angle = match &light.light_type {
                        LighType::Spot(spot_light) => f32::cos(spot_light.cone_angle),
                        _ => 0.0,
                    };
                }
            }
        }
    }
}
