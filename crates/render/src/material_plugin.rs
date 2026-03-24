/// Generic material plugin.
///
/// Registering `MaterialPlugin::<M>::new()` sets up everything needed to render
/// meshes that use material type `M`:
///
/// * Registers `M` as an asset type.
/// * Creates a `wgpu::RenderPipeline` whose layout is built from `M`'s own bind
///   group layout plus whichever engine bind groups `M` declares it needs via
///   [`Material::needs_camera`], [`Material::needs_lighting`] and
///   [`Material::needs_skeleton`].
/// * Adds a render pass (`material_renderpass<M>`) that only processes mesh
///   entities carrying [`MaterialComponent<M>`].
///
/// # Using a custom material
///
/// 1. Derive (or manually implement) [`AsBindGroup`] for your type, then implement
///    [`Material`] (usually with default methods).
/// 2. Register the plugin: `app.register_plugin(MaterialPlugin::<MyMaterial>::new())`.
/// 3. Attach [`MaterialComponent::<MyMaterial>`] + [`MeshComponent`] to an entity.
///
/// # Shader bind-group convention
///
/// | Group | Contents                         | Condition                      |
/// |-------|----------------------------------|--------------------------------|
/// | 0     | Material's own bindings          | always                         |
/// | 1     | Camera uniform                   | `M::needs_camera()` → true     |
/// | 2     | Lighting uniform                 | `M::needs_lighting()` → true   |
/// | 3     | Skeleton (bone) uniforms         | `M::needs_skeleton()` → true   |
///
/// Your WGSL only needs to declare the groups that your material actually uses.
use std::marker::PhantomData;

use app::plugins::Plugin;
use ecs::{
    command::CommandQueue,
    entity::Entity,
    query::{query_filter::Added, Query},
    resource::{Res, ResMut, Resource},
    system::system_input::SystemInputData,
};
use essential::transform::GlobalTranform;
use wgpu::util::DeviceExt;

use crate::{
    assets::{
        material::{Material, ShaderRef},
    },
    components::{
        camera::RenderCamera,
        light::RenderLights,
        material_component::MaterialComponent,
        mesh_component::{MeshComponent, RenderMeshInstance},
        render_entity::RenderEntity,
        render_material_component::RenderMaterialComponent,
        skeleton_component::{EmptySkeletonBuffer, RenderSkeletonComponent},
    },
    device::RenderDevice,
    layouts::{CameraLayout, LightLayout, SkeletonLayout},
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
///
/// Inserted as a resource by [`MaterialPlugin<M>::finish`].
pub struct MaterialPipeline<M: 'static> {
    pub pipeline: wgpu::RenderPipeline,
    /// The `@group(0)` bind-group layout for `M`'s own data.
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

/// GPU-side representation of a prepared `M` instance.
///
/// Stores the `wgpu::BindGroup` built from the material's data and ready to be
/// bound at `@group(0)` during the material render pass.
pub struct RenderMaterial<M: 'static> {
    pub bind_group: wgpu::BindGroup,
    _marker: PhantomData<fn() -> M>,
}

impl<M: Material + 'static> RenderAsset for RenderMaterial<M> {
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
        Ok(RenderMaterial {
            bind_group,
            _marker: PhantomData,
        })
    }
}

// ─── Systems ──────────────────────────────────────────────────────────────────

/// Creates a render-world instance when a [`MeshComponent`] is added to an
/// entity that also carries [`MaterialComponent<M>`].
pub(crate) fn mesh_added<M: Material>(
    meshes: Query<
        (
            Entity,
            &MeshComponent,
            &MaterialComponent<M>,
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
            label: Some("Material Instance Buffer"),
            contents: bytemuck::cast_slice(&[transform.to_raw()]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let instance = RenderMeshInstance {
            mesh_asset_id: mesh.handle.id(),
            transform: instance_buffer,
        };
        let render_mat = RenderMaterialComponent::<M>::new(material.handle.id());

        match render_entity {
            Some(re) => {
                cmd.insert(instance, **re);
                cmd.insert(render_mat, **re);
            }
            None => {
                let new_re = cmd.spawn((instance, render_mat));
                cmd.insert(RenderEntity::new(new_re), entity);
            }
        }
    }
}

/// Render pass for meshes that use material `M`.
///
/// Only processes entities tagged with [`RenderMaterialComponent<M>`] so
/// multiple `MaterialPlugin` instantiations for different material types can
/// coexist without interfering with each other.
pub(crate) fn material_renderpass<M: Material>(
    pipeline: Res<MaterialPipeline<M>>,
    mut device: ResMut<RenderDevice>,
    render_mesh_query: Query<(
        &RenderMeshInstance,
        Option<&RenderSkeletonComponent>,
        &RenderMaterialComponent<M>,
    )>,
    render_cameras: Query<&RenderCamera>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_materials: Res<RenderAssets<RenderMaterial<M>>>,
    render_window: Res<RenderWindow>,
    render_lights: Res<RenderLights>,
    empty_skeleton: Res<EmptySkeletonBuffer>,
) {
    if let Some(view) = render_window.get_view() {
        let encoder = device.command_encoder();

        for render_camera in render_cameras.iter() {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Material Render Pass"),
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

            // Set the engine built-in bind groups that M declared it needs.
            // Only the groups present in the pipeline layout are set — the
            // layout was built with the same conditions in `MaterialPlugin::finish()`.
            if M::needs_camera() {
                render_pass.set_bind_group(1, &render_camera.camera_bind_group, &[]);
            }
            if M::needs_lighting() {
                render_pass.set_bind_group(2, &render_lights.bind_group, &[]);
            }

            for (mesh_instance, skeleton, render_mat_comp) in render_mesh_query.iter() {
                if let Some(mesh) = render_meshes.get(&mesh_instance.mesh_asset_id) {
                    if let Some(render_mat) =
                        render_materials.get(&render_mat_comp.material_asset_id)
                    {
                        render_pass.set_bind_group(0, &render_mat.bind_group, &[]);
                    } else {
                        continue;
                    }

                    if M::needs_skeleton() {
                        if let Some(sk) = skeleton {
                            render_pass.set_bind_group(3, &sk.skeleton_bind_group, &[]);
                        } else {
                            render_pass.set_bind_group(3, &**empty_skeleton, &[]);
                        }
                    }

                    render_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    render_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.set_vertex_buffer(1, mesh_instance.transform.slice(..));
                    render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                }
            }
        }
    }
}

// ─── MaterialPlugin ───────────────────────────────────────────────────────────

/// Register this plugin to enable rendering with material `M`.
///
/// ```rust,ignore
/// app.register_plugin(MaterialPlugin::<UnlitMaterial>::new());
/// ```
///
/// The plugin uses `M`'s [`Material`] flags to build a pipeline layout that
/// contains only the bind groups the shaders actually need.  Shader code
/// therefore never has to declare unused `@group(N)` bindings.
///
/// # Pipeline-only mode
///
/// For materials that have their own custom render pass (e.g. skybox, UI)
/// you can use [`MaterialPlugin::pipeline_only`] to create just the
/// [`MaterialPipeline<M>`] resource without registering the generic mesh
/// rendering systems.
pub struct MaterialPlugin<M: Material> {
    /// When `true`, only the [`MaterialPipeline<M>`] resource is set up.
    /// No asset registration and no mesh rendering systems are added.
    pipeline_only: bool,
    _marker: PhantomData<fn() -> M>,
}

impl<M: Material> Default for MaterialPlugin<M> {
    fn default() -> Self {
        Self {
            pipeline_only: false,
            _marker: PhantomData,
        }
    }
}

impl<M: Material> MaterialPlugin<M> {
    /// Full material plugin: registers the asset type, the render-asset
    /// pipeline, and the generic mesh rendering systems.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pipeline-only mode: only creates the [`MaterialPipeline<M>`] resource.
    ///
    /// Use this when the material has its own custom render pass (e.g. skybox,
    /// UI) and you only need the wgpu pipeline to be built from the material's
    /// trait methods without the generic mesh rendering systems.
    pub fn pipeline_only() -> Self {
        Self {
            pipeline_only: true,
            _marker: PhantomData,
        }
    }
}

impl<M: Material> Plugin for MaterialPlugin<M> {
    fn build(&self, app: &mut app::App) {
        if self.pipeline_only {
            // Pipeline-only: no asset store, no render-asset preparation, no
            // mesh rendering systems.  Only the pipeline resource is created
            // in `finish`.
            return;
        }

        app.register_asset::<M>();
        app.register_plugin(RenderAssetPlugin::<RenderMaterial<M>>::new());

        // mesh_added<M> must be per-material so we know which material handle to store
        // in RenderMaterialComponent<M>.  Transform updates, however, are handled by the
        // shared mesh_changed system already registered by RenderPlugin, which iterates
        // over all entities with RenderEntity regardless of material type.
        app.add_system(app::update_group::UpdateGroup::LateUpdate, mesh_added::<M>)
            .add_system(
                app::update_group::UpdateGroup::Render,
                material_renderpass::<M>,
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

        // Build the material's own @group(0) bind-group layout.
        let material_layout = M::bind_group_layout(device);

        // Gather only the engine layouts that M's shaders declare they need,
        // in fixed slot order: camera(1), lighting(2), skeleton(3).
        // Using the exact same BGL objects stored in resources ensures the
        // bind groups created from them are compatible with the pipeline.
        let camera_layout_res = if M::needs_camera() {
            Some(
                app.get_resource::<CameraLayout>()
                    .expect("CameraLayout not found"),
            )
        } else {
            None
        };
        let light_layout_res = if M::needs_lighting() {
            Some(
                app.get_resource::<LightLayout>()
                    .expect("LightLayout not found"),
            )
        } else {
            None
        };
        let skeleton_layout_res = if M::needs_skeleton() {
            Some(
                app.get_resource::<SkeletonLayout>()
                    .expect("SkeletonLayout not found"),
            )
        } else {
            None
        };

        let mut all_layouts: Vec<&wgpu::BindGroupLayout> = vec![&material_layout];
        if let Some(cl) = &camera_layout_res {
            all_layouts.push(&cl.camera_layout);
        }
        if let Some(ll) = &light_layout_res {
            all_layouts.push(ll);
        }
        if let Some(sl) = &skeleton_layout_res {
            all_layouts.push(sl);
        }

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Material Pipeline Layout"),
            bind_group_layouts: &all_layouts,
            push_constant_ranges: &[],
        });

        // Resolve shader sources (fall back to the built-in Phong shader).
        let vs_src: &str = match M::vertex_shader() {
            ShaderRef::Default => DEFAULT_SHADER_SOURCE,
            ShaderRef::Source(src) => src,
        };
        let fs_src: &str = match M::fragment_shader() {
            ShaderRef::Default => DEFAULT_SHADER_SOURCE,
            ShaderRef::Source(src) => src,
        };

        let vs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Material VS"),
            source: wgpu::ShaderSource::Wgsl(vs_src.into()),
        });
        let fs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Material FS"),
            source: wgpu::ShaderSource::Wgsl(fs_src.into()),
        });

        // Use the vertex layouts from the material trait.
        let vertex_layouts = M::vertex_layouts();

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Material Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: Some("vs_main"),
                buffers: &vertex_layouts,
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
                cull_mode: M::cull_mode(),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: M::depth_stencil(),
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
