use crate::{
    assets::{mesh::Mesh, skeleton::Skeleton, texture::Texture},
    components::{
        camera::{camera_added, camera_changed},
        light::{light_added, light_changed, prepare_lights_buffer, RenderLights},
        mesh_component::{mesh_added, mesh_changed},
        render_entity::RenderEntity,
        skeleton_component::{skeleton_added, update_skeletons, EmptySkeletonBuffer},
        world_environment::WorldEnvironment,
    },
    device::RenderDevice,
    layouts::{CameraLayout, LightLayout, SkeletonLayout},
    material_plugin::clear_cameras,
    queue::RenderQueue,
    render_asset::{
        render_mesh::RenderMesh,
        render_texture::{DummyRenderTexture, RenderTexture},
        render_window::RenderWindow,
        RenderAssetPlugin,
    },
    resources::RenderContext,
    systems::{
        render::{finish_render, present_window},
        update_window,
    },
};
use app::plugins::Plugin;
use color::LinearRgba;
use ecs::{resource::Resource, system::schedule::UpdateGroup, IntoSystemConfig};
use std::sync::{Arc, Mutex};
use wgpu::{Adapter, Device, Instance, Limits, MemoryHints, Queue};

pub struct RenderResources {
    pub device: Device,
    pub queue: Queue,
    pub adapter: Adapter,
    pub instance: Instance,
    pub surface: Option<Arc<wgpu::Surface<'static>>>,
}

#[derive(Resource)]
struct FutureRenderResources(Arc<Mutex<Option<RenderResources>>>);

pub struct RenderPlugin;

impl RenderPlugin {
    async fn initialize_renderer(
        window_handle: Option<Arc<winit::window::Window>>,
    ) -> RenderResources {
        let instance: Instance = wgpu::Instance::default();

        let surface = window_handle.as_ref().map(|handle| {
            Arc::new(
                instance
                    .create_surface(Arc::clone(handle))
                    .expect("Error creating surface."),
            )
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: surface.as_deref(),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: if cfg!(target_arch = "wasm32") {
                        Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                    memory_hints: MemoryHints::Performance,
                },
                None,
            )
            .await
            .unwrap();

        RenderResources {
            device,
            queue,
            adapter,
            instance,
            surface,
        }
    }
}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut app::App) {
        let future_render_resources_wrapper = Arc::new(Mutex::new(None));
        app.insert_resource(FutureRenderResources(
            future_render_resources_wrapper.clone(),
        ));

        let is_windowed = app.get_resource::<window::plugin::Window>().is_some();

        if is_windowed {
            let window = app.get_resource::<window::plugin::Window>().unwrap();
            let window_handle = Arc::clone(&window.window_handle);

            let async_init = async move {
                let resources = RenderPlugin::initialize_renderer(Some(window_handle)).await;
                *future_render_resources_wrapper.lock().unwrap() = Some(resources);
            };

            #[cfg(target_arch = "wasm32")]
            wasm_bindgen_futures::spawn_local(async_init);
            #[cfg(not(target_arch = "wasm32"))]
            pollster::block_on(async_init);
        } else {
            let async_init = async move {
                let resources = RenderPlugin::initialize_renderer(None).await;
                *future_render_resources_wrapper.lock().unwrap() = Some(resources);
            };
            pollster::block_on(async_init);
        }

        app.register_plugin(RenderAssetPlugin::<RenderMesh>::new())
            .register_plugin(RenderAssetPlugin::<RenderTexture>::new());
        app.register_asset::<Mesh>()
            .register_asset::<Texture>()
            .register_asset::<Skeleton>();

        app.add_system(UpdateGroup::LateUpdate, camera_added)
            .add_system(UpdateGroup::LateUpdate, camera_changed)
            .add_system(UpdateGroup::LateUpdate, mesh_added)
            .add_system(UpdateGroup::LateUpdate, mesh_changed)
            .add_system(UpdateGroup::LateUpdate, light_added)
            .add_system(UpdateGroup::LateUpdate, light_changed)
            .add_system(UpdateGroup::LateUpdate, skeleton_added);

        if is_windowed {
            app.add_system(UpdateGroup::Update, update_window::request_window_resize)
                .add_system(UpdateGroup::Render, update_window::update_render_window);
        }

        app.add_system(UpdateGroup::Render, clear_cameras)
            .add_system(UpdateGroup::Render, update_skeletons)
            .add_system(UpdateGroup::Render, prepare_lights_buffer)
            .add_system(UpdateGroup::LateRender, present_window.after(finish_render));
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
        let RenderResources {
            device,
            queue,
            adapter,
            instance: _instance,
            surface,
        } = app
            .remove_resource::<FutureRenderResources>()
            .unwrap()
            .0
            .lock()
            .unwrap()
            .take()
            .unwrap();

        let config = if let Some(ref surface) = surface {
            let surface_caps = surface.get_capabilities(&adapter);
            let surface_format = surface_caps
                .formats
                .iter()
                .find(|f| f.is_srgb())
                .copied()
                .unwrap_or(surface_caps.formats[0]);

            let window = app.get_resource::<window::plugin::Window>().unwrap();
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

            surface.configure(&device, &config);
            config
        } else {
            let surface_format = wgpu::TextureFormat::Rgba8Unorm;
            wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width: 0,
                height: 0,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            }
        };

        let camera_layouts = CameraLayout::new(&device);

        let light_layout = LightLayout::new(&device);

        let skeleton_layout = SkeletonLayout::new(&device);

        app.register_component_lifecycle::<RenderEntity>();

        let render_lights = RenderLights::new(&device, &light_layout);
        let empty_skeleton_buffer = EmptySkeletonBuffer::new(&device, &skeleton_layout);

        app.insert_resource(DummyRenderTexture::new(&device))
            .insert_resource(RenderContext {
                surface,
                surface_config: config,
            })
            .insert_resource(RenderDevice {
                device,
                encoder: None,
            })
            .insert_resource(RenderQueue { queue })
            .insert_resource(RenderWindow::new())
            .insert_resource(camera_layouts)
            .insert_resource(light_layout)
            .insert_resource(skeleton_layout)
            .insert_resource(render_lights)
            .insert_resource(empty_skeleton_buffer)
            .insert_resource(WorldEnvironment::new(LinearRgba::new(0.1, 0.1, 0.1, 0.1)));
    }
}
