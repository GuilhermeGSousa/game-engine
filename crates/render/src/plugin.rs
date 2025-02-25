use std::sync::Arc;

use app::plugin::Plugin;
use bevy_ecs::prelude::*;

use window::plugin::Window;

pub struct RenderPlugin;

#[derive(Resource)]
struct RenderSurface(Arc<wgpu::Surface<'static>>);

#[derive(Resource)]
struct RenderAdapter(wgpu::Adapter);

#[derive(Resource)]
struct RenderDevice(wgpu::Device);

#[derive(Resource)]
struct RenderQueue(wgpu::Queue);

#[derive(Resource)]
struct RenderConfig(wgpu::SurfaceConfiguration);

fn render(
    mut window: ResMut<Window>,
    surface: Res<RenderSurface>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut config: ResMut<RenderConfig>,
) {
    if window.should_resize() {
        let size = window.size();
        let surface = surface.0.clone();
        config.0.width = size.0;
        config.0.height = size.1;
        surface.configure(&device.0, &config.0);
        window.clear_resize();
    }

    if let Ok(output) = surface.0.get_current_texture() {
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device
            .0
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        queue.0.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut app::App) {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let window = app.resource::<Window>();
        let size = window.size();

        let surface = Arc::new(
            instance
                .create_surface(Arc::clone(&window.window_handle))
                .expect("Error creating surface."),
        );

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web, we'll have to disable some.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                label: None,
                memory_hints: Default::default(),
            },
            None, // Trace path
        ))
        .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

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

        app.insert_resource(RenderSurface(surface));
        app.insert_resource(RenderAdapter(adapter));
        app.insert_resource(RenderDevice(device));
        app.insert_resource(RenderQueue(queue));
        app.insert_resource(RenderConfig(config));
        app.add_system(app::update_group::UpdateGroup::Update, render);
    }
}
