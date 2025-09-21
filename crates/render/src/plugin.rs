use crate::{
    assets::{
        material::Material,
        mesh::Mesh,
        scene::Scene,
        texture::Texture,
        vertex::{Vertex, VertexBufferLayout},
    },
    components::{
        light::{prepare_lights_buffer, RenderLights},
        mesh_component::{mesh_added, mesh_changed},
        render_entity::RenderEntity,
        skybox::{prepare_skybox, RenderSkyboxCube, SkyboxVertex},
        world_environment::WorldEnvironment,
    },
    device::RenderDevice,
    layouts::{CameraLayouts, LightLayouts, MaterialLayouts},
    loaders::{
        gltf_loader::{spawn_gltf_component, GLTFScene},
        obj_loader::{spawn_obj_component, OBJAsset},
    },
    queue::RenderQueue,
    render_asset::{
        render_material::RenderMaterial,
        render_mesh::RenderMesh,
        render_texture::{DummyRenderTexture, RenderTexture},
        render_window::RenderWindow,
        RenderAssetPlugin,
    },
    resources::RenderContext,
    systems::{
        render::{self, present_window},
        sync_entities::{camera_added, camera_changed, light_added, light_changed},
        update_window,
    },
};
use app::plugins::Plugin;
use ecs::resource::Resource;
use essential::transform::GlobalTransformRaw;
use glam::Vec4;
use std::sync::{Arc, Mutex};
use wgpu::{
    Adapter, Device, FragmentState, Instance, Limits, MemoryHints, Queue, RenderPipelineDescriptor,
};
use window::plugin::Window;
use winit::window::Window as WinitWindow;

pub struct RenderResources(
    pub Device,
    pub Queue,
    pub Adapter,
    pub Instance,
    Arc<wgpu::Surface<'static>>,
);

#[derive(Resource)]
struct FutureRenderResources(Arc<Mutex<Option<RenderResources>>>);

pub struct RenderPlugin;

impl RenderPlugin {
    async fn initialize_renderer(window_handle: Arc<WinitWindow>) -> RenderResources {
        let instance: Instance = wgpu::Instance::default();

        let surface: Arc<wgpu::Surface<'_>> = Arc::new(
            instance
                .create_surface(Arc::clone(&window_handle))
                .expect("Error creating surface."),
        );

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web, we'll have to disable some.
                    required_limits: if cfg!(target_arch = "wasm32") {
                        Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                    memory_hints: MemoryHints::Performance,
                },
                None, // Trace path
            )
            .await
            .unwrap();
        RenderResources(device, queue, adapter, instance, surface)
    }
}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut app::App) {
        let future_render_resources_wrapper = Arc::new(Mutex::new(None));
        app.insert_resource(FutureRenderResources(
            future_render_resources_wrapper.clone(),
        ));

        let window = app.get_resource::<Window>().unwrap();
        let window_handle = Arc::clone(&window.window_handle);
        let async_renderer_initialization = async move {
            let resources = RenderPlugin::initialize_renderer(window_handle).await;
            *future_render_resources_wrapper.lock().unwrap() = Some(resources);
        };

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async_renderer_initialization);

        #[cfg(not(target_arch = "wasm32"))]
        pollster::block_on(async_renderer_initialization);

        app.register_plugin(RenderAssetPlugin::<RenderMesh>::new())
            .register_plugin(RenderAssetPlugin::<RenderTexture>::new())
            .register_plugin(RenderAssetPlugin::<RenderMaterial>::new());

        app.register_asset::<Mesh>()
            .register_asset::<Texture>()
            .register_asset::<Material>()
            .register_asset::<Scene>();

        app.add_system(app::update_group::UpdateGroup::LateUpdate, camera_added)
            .add_system(app::update_group::UpdateGroup::LateUpdate, camera_changed)
            .add_system(app::update_group::UpdateGroup::LateUpdate, mesh_added)
            .add_system(app::update_group::UpdateGroup::LateUpdate, mesh_changed)
            .add_system(app::update_group::UpdateGroup::LateUpdate, light_added)
            .add_system(app::update_group::UpdateGroup::LateUpdate, light_changed)
            .add_system(
                app::update_group::UpdateGroup::Update,
                update_window::request_window_resize,
            )
            .add_system(app::update_group::UpdateGroup::Update, spawn_obj_component)
            .add_system(app::update_group::UpdateGroup::Update, spawn_gltf_component)
            .add_system(
                app::update_group::UpdateGroup::Render,
                update_window::update_render_window,
            )
            .add_system(app::update_group::UpdateGroup::Render, prepare_skybox)
            .add_system(
                app::update_group::UpdateGroup::Render,
                prepare_lights_buffer,
            )
            .add_system(
                app::update_group::UpdateGroup::Render,
                render::skybox_renderpass,
            )
            .add_system(
                app::update_group::UpdateGroup::Render,
                render::main_renderpass,
            )
            .add_system(
                app::update_group::UpdateGroup::Render,
                render::finish_render,
            )
            .add_system(app::update_group::UpdateGroup::LateRender, present_window);
    }

    fn ready(&self, app: &app::App) -> bool {
        app.get_resource::<FutureRenderResources>()
            .and_then(|future_render_resources| {
                future_render_resources
                    .0
                    .try_lock()
                    .map(|mutex| mutex.is_some())
                    .ok()
            })
            .unwrap_or(true)
    }

    fn finish(&self, app: &mut app::App) {
        let RenderResources(device, queue, adapter, _instance, surface) = app
            .remove_resource::<FutureRenderResources>()
            .unwrap()
            .0
            .lock()
            .unwrap()
            .take()
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let window = app.get_resource::<Window>().unwrap();
        let size = window.size();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.0,
            height: size.1,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let main_shader = device.create_shader_module(wgpu::include_wgsl!("shaders\\shader.wgsl"));
        let skybox_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders\\skybox.wgsl"));

        let camera_layouts = CameraLayouts::new(&device);

        let material_layouts = MaterialLayouts::new(&device);

        let light_layouts = LightLayouts::new(&device);

        let skybox_cube = RenderSkyboxCube::new(&device);

        // Setup render pipeline
        let main_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &material_layouts.main_material_layout,
                    &camera_layouts.camera_layout,
                    &light_layouts.lights_layout,
                ],
                push_constant_ranges: &[],
            });

        let main_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&main_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &main_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::describe(), GlobalTransformRaw::describe()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                // 3.
                module: &main_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    // 4.
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
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

        let skybox_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Skybox Pipeline Layout"),
                bind_group_layouts: &[
                    &camera_layouts.camera_layout,
                    &material_layouts.skybox_material_layout,
                ],
                push_constant_ranges: &[],
            });

        let skybox_render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Skybox Pipeline"),
            layout: Some(&skybox_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &skybox_shader,
                entry_point: Some("vs_main"),
                buffers: &[SkyboxVertex::describe()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Front),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &skybox_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
            cache: None,
        });

        app.register_component_lifecycle::<RenderEntity>();

        let render_lights = RenderLights::new(&device, &light_layouts);

        app.insert_resource(DummyRenderTexture::new(&device))
            .insert_resource(RenderContext {
                surface: surface,
                surface_config: config,
                main_pipeline: main_render_pipeline,
                skybox_pipeline: skybox_render_pipeline,
            })
            .insert_resource(RenderDevice {
                device,
                encoder: None,
            })
            .insert_resource(RenderQueue { queue })
            .insert_resource(RenderWindow::new())
            .insert_resource(material_layouts)
            .insert_resource(camera_layouts)
            .insert_resource(light_layouts)
            .insert_resource(render_lights)
            .insert_resource(skybox_cube)
            .insert_resource(WorldEnvironment::new(Vec4::new(0.1, 0.1, 0.1, 0.1)));
    }
}
