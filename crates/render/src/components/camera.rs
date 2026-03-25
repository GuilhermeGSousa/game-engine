use encase::{ShaderType, UniformBuffer};
use essential::{
    assets::{asset_store::AssetStore, handle::AssetHandle},
    transform::GlobalTranform,
};

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
    #[allow(dead_code)]
    CustomWindow(Entity),
}

/// Where a camera renders its output.
pub enum RenderTarget {
    /// Render to a window surface (default: the main application window).
    Window(WindowRef),
    /// Render to the off-screen texture identified by this asset handle.
    ///
    /// The texture must have been created with
    /// [`Texture::new_render_target`] (or with
    /// `RENDER_ATTACHMENT | TEXTURE_BINDING` usage flags set manually) so
    /// that the GPU resource supports both rendering into it and sampling
    /// from it in materials or UI panels.
    ///
    /// The engine will add a [`CameraTextureTarget`] component to the
    /// camera's render entity once the texture asset is available, which
    /// signals all render passes to write into this texture instead of the
    /// main window surface.
    Texture(AssetHandle<Texture>),
}

impl RenderTarget {
    pub fn main_window() -> Self {
        RenderTarget::Window(WindowRef::MainWindow)
    }

    /// Creates a render target that renders into the given off-screen texture.
    ///
    /// `handle` should refer to a [`Texture`] created with
    /// [`Texture::new_render_target`].
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

/// Component added to the camera's render entity when the camera renders to
/// an off-screen texture.
///
/// The asset handle stored here refers to the same [`Texture`] that was
/// passed to [`RenderTarget::texture`].  The corresponding GPU resource lives
/// in [`RenderAssets<RenderTexture>`] and can be looked up with this handle's
/// id to obtain the [`wgpu::TextureView`] for rendering or sampling.
#[derive(Component)]
pub struct CameraTextureTarget {
    /// Asset handle for the off-screen colour texture this camera renders into.
    pub texture_handle: AssetHandle<Texture>,
}

pub(crate) fn camera_added(
    cameras: Query<(Entity, &Camera, &GlobalTranform, Option<&RenderEntity>), Added<(Camera,)>>,
    mut cmd: CommandQueue,
    device: Res<RenderDevice>,
    context: Res<RenderContext>,
    camera_layouts: Res<CameraLayout>,
    texture_assets: Res<AssetStore<Texture>>,
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

        // For texture render targets, size the depth texture to match the colour
        // target.  For window cameras, use the current surface dimensions.
        let (depth_texture, texture_target) = match &camera.render_target {
            RenderTarget::Window(_) => {
                let depth = RenderTexture::create_depth_texture(
                    &device,
                    &context.surface_config,
                    "depth_texture",
                );
                (depth, None)
            }
            RenderTarget::Texture(handle) => {
                let (width, height) = texture_assets
                    .get(handle)
                    .map(|t| {
                        let s = t.size();
                        (s.width, s.height)
                    })
                    .unwrap_or((context.surface_config.width, context.surface_config.height));

                let depth = RenderTexture::create_depth_texture_sized(
                    &device,
                    width,
                    height,
                    "depth_texture",
                );
                let target = CameraTextureTarget {
                    texture_handle: handle.clone(),
                };
                (depth, Some(target))
            }
        };

        let render_cam = RenderCamera {
            camera_bind_group,
            camera_uniform,
            camera_buffer,
            depth_texture,
            clear_color: camera.clear_color,
        };

        match render_entity {
            None => {
                let new_render_entity = cmd.spawn(render_cam);
                cmd.insert(RenderEntity::new(new_render_entity), entity);
                if let Some(target) = texture_target {
                    // cmd.insert on new_render_entity — need to capture it
                    // We re-query via the stored new_render_entity handle:
                    cmd.insert(target, new_render_entity);
                }
            }
            Some(render_entity) => {
                cmd.insert(render_cam, **render_entity);
                if let Some(target) = texture_target {
                    cmd.insert(target, **render_entity);
                }
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
