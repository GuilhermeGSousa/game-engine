use ecs::resource::Resource;
use wgpu::{TextureView, TextureViewDescriptor};

#[derive(Resource)]
pub struct RenderWindow {
    view: Option<TextureView>,
    texture: Option<wgpu::SurfaceTexture>,
}

impl RenderWindow {
    pub fn new() -> Self {
        RenderWindow {
            view: None,
            texture: None,
        }
    }

    pub fn set_swapchain_texture(&mut self, frame: wgpu::SurfaceTexture) {
        self.view = Some(frame.texture.create_view(&TextureViewDescriptor::default()));
        self.texture = Some(frame);
    }

    pub fn get_view(&self) -> Option<&TextureView> {
        self.view.as_ref()
    }

    pub fn present(&mut self) {
        if let Some(texture) = self.texture.take() {
            texture.present();
            self.view = None;
        }
    }
}
