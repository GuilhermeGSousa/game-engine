use core::resource::Resource;

use egui::Context;
use egui_wgpu::{
    wgpu::{Device, TextureFormat},
    Renderer,
};
use egui_winit::{winit::window::Window, State};

#[derive(Resource)]
pub struct UIRenderer {
    pub context: Context,
    pub state: State,
    pub renderer: Renderer,
}

impl UIRenderer {
    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        dithering: bool,
        window: &Window,
    ) -> Self {
        let egui_context = Context::default();
        let id = egui_context.viewport_id();

        let egui_state = State::new(egui_context.clone(), id, &window, None, None, None);
        let egui_renderer = Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
            dithering,
        );

        UIRenderer {
            context: egui_context,
            state: egui_state,
            renderer: egui_renderer,
        }
    }
}

#[derive(Resource)]
pub struct UIWindow(pub Window);
