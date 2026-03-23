/// Generic material plugin.
///
/// Registering `MaterialPlugin::<M>::new()` with the app sets up everything
/// needed to render meshes that use the custom material type `M`:
///
/// * Registers `M` as an asset type.
/// * Creates a `wgpu::RenderPipeline` from `M::bind_group_layout()` and the
///   shader sources returned by `M::vertex_shader()` / `M::fragment_shader()`.
/// * Adds a [`CustomMaterialComponent<M>`] component so users can attach `M`
///   to an entity alongside a [`MeshComponent`].
/// * Adds all necessary render systems (mesh added / changed / render pass).
///
/// # Shader convention
///
/// The vertex shader **must** accept the same vertex inputs as the built-in
/// shader (see `crates/render/src/shaders/shader.wgsl`) and bind the camera
/// uniform at `@group(1) @binding(0)`.  The material's own bindings go in
/// `@group(0)`.  Lights and the skeleton buffer are bound at groups 2 and 3
/// respectively; they don't need to be declared in the shader if unused.
use std::marker::PhantomData;

use app::plugins::Plugin;
use ecs::{
    command::CommandQueue,
    component::Component,
    entity::Entity,
    query::{
        query_filter::{Added, Changed},
        Query,
    },
    resource::{Res, ResMut, Resource},
    system::system_input::SystemInputData,
};
use essential::{
    assets::{handle::AssetHandle, Asset},
    transform::{GlobalTranform, GlobalTransformRaw, Transform},
};
use wgpu::util::DeviceExt;

use crate::{
    assets::{
        material::{AsBindGroup, ShaderRef},
        vertex::{Vertex, VertexBufferLayout},
    },
    components::{
        camera::RenderCamera,
        light::RenderLights,
        mesh_component::{MeshComponent, RenderMeshInstance},
        render_entity::RenderEntity,
        skeleton_component::{EmptySkeletonBuffer, RenderSkeletonComponent},
    },
    device::RenderDevice,
    layouts::{CameraLayout, LightLayout, SkeletonLayout},
    queue::RenderQueue,
    render_asset::{
        render_mesh::RenderMesh,
        render_texture::{DummyRenderTexture, RenderTexture},
        render_window::RenderWindow,
        AssetPreparationError, RenderAsset, RenderAssetPlugin, RenderAssets,
    },
    resources::RenderContext,
};

// ─── Default (built-in) shader source ────────────────────────────────────────

const DEFAULT_SHADER_SOURCE: &str = include_str!("shaders/shader.wgsl");

// ─── MaterialPipeline ─────────────────────────────────────────────────────────

/// Stores the wgpu render pipeline and material bind-group layout for `M`.
pub struct MaterialPipeline<M: 'static> {
    pub pipeline: wgpu::RenderPipeline,
    /// Layout used when calling `M::create_bind_group`.
    pub bind_group_layout: wgpu::BindGroupLayout,
    _marker: PhantomData<fn() -> M>,
}

impl<M: 'static> MaterialPipeline<M> {
    pub fn new(pipeline: wgpu::RenderPipeline, bind_group_layout: wgpu::BindGroupLayout) -> Self {
        Self {
            pipeline,
            bind_group_layout,
            _marker: PhantomData,
        }
    }
}

// Manual Resource impl — #[derive(Resource)] doesn't handle PhantomData<fn()>.
impl<M: 'static> Resource for MaterialPipeline<M> {
    fn name() -> &'static str {
        std::any::type_name::<MaterialPipeline<M>>()
    }
}

// ─── CustomMaterialComponent ──────────────────────────────────────────────────

/// Attach this component (alongside [`MeshComponent`]) to spawn a mesh that
/// uses material `M` instead of [`StandardMaterial`].
pub struct CustomMaterialComponent<M: Asset + 'static> {
    pub handle: AssetHandle<M>,
}

// Manual Component impl — #[derive(Component)] does not support generics.
impl<M: Asset + Send + Sync + 'static> Component for CustomMaterialComponent<M> {
    fn name() -> &'static str {
        std::any::type_name::<CustomMaterialComponent<M>>()
    }
}

// ─── CustomMaterialTag ────────────────────────────────────────────────────────

/// Marker placed on *render* entities to distinguish them from standard-material
/// entities.  Queries in [`custom_renderpass`] filter on this component so that
/// only custom-material mesh instances are processed by the custom pipeline.
pub(crate) struct CustomMaterialTag<M: 'static> {
    _marker: PhantomData<fn() -> M>,
}

impl<M: 'static> CustomMaterialTag<M> {
    fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<M: Send + Sync + 'static> Component for CustomMaterialTag<M> {
    fn name() -> &'static str {
        std::any::type_name::<CustomMaterialTag<M>>()
    }
}

// ─── RenderCustomMaterial ─────────────────────────────────────────────────────

/// GPU-side representation of a prepared `M` instance.
///
/// Stores the [`wgpu::BindGroup`] built from the material's data and ready to
/// be bound at `@group(0)` during the custom render pass.
pub struct RenderCustomMaterial<M: 'static> {
    pub bind_group: wgpu::BindGroup,
    _marker: PhantomData<fn() -> M>,
}

impl<M: AsBindGroup + Asset + Send + Sync + 'static> RenderAsset for RenderCustomMaterial<M> {
    type SourceAsset = M;
    type PreparationParams = (
        Res<'static, RenderDevice>,
        Res<'static, RenderAssets<RenderTexture>>,
        Res<'static, DummyRenderTexture>,
        Res<'static, MaterialPipeline<M>>,
    );

    fn prepare_asset(
        source_asset: &Self::SourceAsset,
        params: &mut SystemInputData<Self::PreparationParams>,
    ) -> Result<Self, AssetPreparationError> {
        let (device, render_textures, dummy_texture, pipeline) = params;
        let bind_group = source_asset.create_bind_group(
            device,
            render_textures,
            dummy_texture,
            &pipeline.bind_group_layout,
        )?;
        Ok(RenderCustomMaterial {
            bind_group,
            _marker: PhantomData,
        })
    }
}

// ─── Systems ──────────────────────────────────────────────────────────────────

/// Like `mesh_added`, but for entities carrying [`CustomMaterialComponent<M>`].
pub(crate) fn custom_mesh_added<M: AsBindGroup + Asset + Send + Sync + 'static>(
    meshes: Query<
        (
            Entity,
            &MeshComponent,
            &CustomMaterialComponent<M>,
            &GlobalTranform,
            Option<&RenderEntity>,
        ),
        Added<(MeshComponent,)>,
    >,
    mut cmd: CommandQueue,
    device: Res<RenderDevice>,
) {
    for (entity, mesh, material, transform, render_entity) in meshes.iter() {
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Custom Material Instance Buffer"),
            contents: bytemuck::cast_slice(&[transform.to_raw()]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let instance = RenderMeshInstance {
            mesh_asset_id: mesh.handle.id(),
            material_asset_id: material.handle.id(),
            transform: instance_buffer,
        };

        match render_entity {
            Some(re) => {
                cmd.insert(instance, **re);
                cmd.insert(CustomMaterialTag::<M>::new(), **re);
            }
            None => {
                let new_re = cmd.spawn((instance, CustomMaterialTag::<M>::new()));
                cmd.insert(RenderEntity::new(new_re), entity);
            }
        }
    }
}

/// Updates the instance transform buffer when the `Transform` changes.
pub(crate) fn custom_mesh_changed<M: AsBindGroup + Asset + Send + Sync + 'static>(
    meshes: Query<(&MeshComponent, &GlobalTranform, &RenderEntity), Changed<(Transform,)>>,
    render_meshes: Query<(&mut RenderMeshInstance, &CustomMaterialTag<M>)>,
    queue: Res<RenderQueue>,
) {
    for (_, transform, render_entity) in meshes.iter() {
        if let Some((render_mesh, _)) = render_meshes.get_entity(**render_entity) {
            queue.write_buffer(
                &render_mesh.transform,
                0,
                bytemuck::cast_slice(&[transform.to_raw()]),
            );
        }
    }
}

/// Render pass for meshes using material `M`.
pub(crate) fn custom_renderpass<M: AsBindGroup + Asset + Send + Sync + 'static>(
    pipeline: Res<MaterialPipeline<M>>,
    mut device: ResMut<RenderDevice>,
    render_mesh_query: Query<(
        &RenderMeshInstance,
        Option<&RenderSkeletonComponent>,
        &CustomMaterialTag<M>,
    )>,
    render_cameras: Query<&RenderCamera>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_materials: Res<RenderAssets<RenderCustomMaterial<M>>>,
    render_window: Res<RenderWindow>,
    render_lights: Res<RenderLights>,
    empty_skeleton: Res<EmptySkeletonBuffer>,
) {
    if let Some(view) = render_window.get_view() {
        let encoder = device.command_encoder();

        for render_camera in render_cameras.iter() {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Custom Material Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &render_camera.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&pipeline.pipeline);
            render_pass.set_bind_group(1, &render_camera.camera_bind_group, &[]);
            render_pass.set_bind_group(2, &render_lights.bind_group, &[]);

            for (mesh_instance, skeleton, _tag) in render_mesh_query.iter() {
                if let Some(mesh) = render_meshes.get(&mesh_instance.mesh_asset_id) {
                    if let Some(render_mat) =
                        render_materials.get(&mesh_instance.material_asset_id)
                    {
                        render_pass.set_bind_group(0, &render_mat.bind_group, &[]);
                    } else {
                        continue;
                    }

                    if let Some(skeleton) = skeleton {
                        render_pass.set_bind_group(3, &skeleton.skeleton_bind_group, &[]);
                    } else {
                        render_pass.set_bind_group(3, &**empty_skeleton, &[]);
                    }

                    render_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    render_pass.set_index_buffer(
                        mesh.indices.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    render_pass.set_vertex_buffer(1, mesh_instance.transform.slice(..));
                    render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                }
            }
        }
    }
}

// ─── MaterialPlugin ───────────────────────────────────────────────────────────

/// Register this plugin to enable rendering with custom material `M`.
///
/// ```rust,ignore
/// app.register_plugin(MaterialPlugin::<UnlitMaterial>::new());
/// ```
pub struct MaterialPlugin<M: AsBindGroup + Asset + Send + Sync + 'static> {
    _marker: PhantomData<fn() -> M>,
}

impl<M: AsBindGroup + Asset + Send + Sync + 'static> Default for MaterialPlugin<M> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<M: AsBindGroup + Asset + Send + Sync + 'static> MaterialPlugin<M> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<M: AsBindGroup + Asset + Send + Sync + 'static> Plugin for MaterialPlugin<M> {
    fn build(&self, app: &mut app::App) {
        app.register_asset::<M>();
        app.register_plugin(RenderAssetPlugin::<RenderCustomMaterial<M>>::new());

        app.add_system(
            app::update_group::UpdateGroup::LateUpdate,
            custom_mesh_added::<M>,
        )
        .add_system(
            app::update_group::UpdateGroup::LateUpdate,
            custom_mesh_changed::<M>,
        )
        .add_system(
            app::update_group::UpdateGroup::Render,
            custom_renderpass::<M>,
        );
    }

    fn finish(&self, app: &mut app::App) {
        let device = app
            .get_resource::<RenderDevice>()
            .expect("RenderDevice not found; register RenderPlugin before MaterialPlugin");
        let surface_format = app
            .get_resource::<RenderContext>()
            .expect("RenderContext not found")
            .surface_config
            .format;
        let camera_layout = app
            .get_resource::<CameraLayout>()
            .expect("CameraLayout not found");
        let light_layout = app
            .get_resource::<LightLayout>()
            .expect("LightLayout not found");
        let skeleton_layout = app
            .get_resource::<SkeletonLayout>()
            .expect("SkeletonLayout not found");

        // Build the material bind-group layout from the concrete material type.
        let material_layout = M::bind_group_layout(device);

        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Custom Material Pipeline Layout"),
                bind_group_layouts: &[
                    &material_layout,
                    &camera_layout.camera_layout,
                    &**light_layout,
                    &**skeleton_layout,
                ],
                push_constant_ranges: &[],
            });

        // Resolve shader sources: fall back to built-in Phong shader when Default.
        let vs_src: &str = match M::vertex_shader() {
            ShaderRef::Default => DEFAULT_SHADER_SOURCE,
            ShaderRef::Source(src) => src,
        };
        let fs_src: &str = match M::fragment_shader() {
            ShaderRef::Default => DEFAULT_SHADER_SOURCE,
            ShaderRef::Source(src) => src,
        };

        // Optimisation: when both shaders live in the same source file, we
        // only need one ShaderModule.
        let vs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Custom Material VS"),
            source: wgpu::ShaderSource::Wgsl(vs_src.into()),
        });

        // We cannot share a &ShaderModule reference across two different
        // lifetime contexts, so always create the fragment module separately.
        // If the source is identical, the GPU driver will cache the SPIR-V.
        let fs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Custom Material FS"),
            source: wgpu::ShaderSource::Wgsl(fs_src.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Custom Material Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::describe(), GlobalTransformRaw::describe()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        app.insert_resource(MaterialPipeline::<M>::new(pipeline, material_layout));
    }
}
