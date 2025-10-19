use encase::{ShaderType, UniformBuffer};
use essential::{assets::handle::AssetHandle, transform::GlobalTranform};

use ecs::{
    command::CommandQueue,
    component::Component,
    entity::Entity,
    query::{query_filter::Added, Query},
    resource::Res,
};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::{
    assets::texture::Texture, components::render_entity::RenderEntity, device::RenderDevice,
    layouts::CameraLayout, queue::RenderQueue, render_asset::render_texture::RenderTexture,
    resources::RenderContext,
};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4::from_cols_array(
    &[
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0
    ]
);

pub enum WindowRef {
    MainWindow,
    CustomWindow(Entity),
}

#[allow(dead_code)]
pub enum RenderTarget {
    Window(WindowRef),
    Texture(AssetHandle<Texture>),
}

#[allow(dead_code)]
impl RenderTarget {
    pub fn main_window() -> Self {
        RenderTarget::Window(WindowRef::MainWindow)
    }

    pub fn custom_window(entity: Entity) -> Self {
        RenderTarget::Window(WindowRef::CustomWindow(entity))
    }

    pub fn texture(handle: AssetHandle<Texture>) -> Self {
        RenderTarget::Texture(handle)
    }
}

#[derive(Component)]
pub struct Camera {
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
    pub clear_color: wgpu::Color,
    pub render_target: RenderTarget,
}

impl Camera {
    pub fn build_projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fovy.to_radians(), self.aspect, self.znear, self.zfar)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            aspect: 1.0,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
            clear_color: wgpu::Color {
                r: 0.118,
                g: 0.831,
                b: 0.922,
                a: 1.0,
            },
            render_target: RenderTarget::main_window(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, ShaderType)]
pub struct CameraUniform {
    view_pos: Vec3,
    view_proj: Mat4,
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_pos: Vec3::ZERO,
            view_proj: Mat4::IDENTITY,
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, transform: &GlobalTranform) {
        self.view_pos = transform.translation().into();
        self.view_proj =
            OPENGL_TO_WGPU_MATRIX * camera.build_projection_matrix() * transform.matrix().inverse();
    }
}

#[derive(Component)]
pub struct RenderCamera {
    pub(crate) clear_color: wgpu::Color,
    pub camera_bind_group: wgpu::BindGroup,
    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub(crate) depth_texture: RenderTexture,
}

pub(crate) fn camera_added(
    cameras: Query<(Entity, &Camera, &GlobalTranform, Option<&RenderEntity>), Added<(Camera,)>>,
    mut cmd: CommandQueue,
    device: Res<RenderDevice>,
    context: Res<RenderContext>,
    camera_layouts: Res<CameraLayout>,
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
}
