use anyhow::Result;
use essential::assets::handle::AssetHandle;
use render::assets::texture::Texture;
use render::render_asset::render_texture::RenderTexture;
use ecs::resource::Resource;
use std::sync::Arc;

use crate::readback::TextureReadback;

#[derive(Resource)]
pub struct TextureReadbackResource {
    /// Handle to the terminal render target texture
    pub terminal_texture_handle: Option<AssetHandle<Texture>>,

    /// Current readback operation
    current_readback: Option<Arc<TextureReadback>>,

    /// Previous frame's readback result (if available)
    last_pixel_data: Option<Vec<u8>>,

    /// Dimensions of last readback
    last_width: u32,
    last_height: u32,
}

impl TextureReadbackResource {
    pub fn new() -> Self {
        Self {
            terminal_texture_handle: None,
            current_readback: None,
            last_pixel_data: None,
            last_width: 0,
            last_height: 0,
        }
    }

    pub fn set_terminal_texture(&mut self, handle: AssetHandle<Texture>) {
        self.terminal_texture_handle = Some(handle);
    }

    pub fn terminal_render_target(&self) -> Option<&AssetHandle<Texture>> {
        self.terminal_texture_handle.as_ref()
    }

    pub fn set_readback(&mut self, readback: TextureReadback) {
        self.current_readback = Some(Arc::new(readback));
    }

    pub fn get_last_pixel_data(&self) -> Option<&[u8]> {
        self.last_pixel_data.as_deref()
    }

    pub fn get_last_dimensions(&self) -> (u32, u32) {
        (self.last_width, self.last_height)
    }

    pub fn update_last_data(&mut self, data: Vec<u8>, width: u32, height: u32) {
        self.last_pixel_data = Some(data);
        self.last_width = width;
        self.last_height = height;
        self.current_readback = None;
    }

    pub fn has_readback(&self) -> bool {
        self.current_readback.is_some()
    }

    pub fn take_current_readback(&mut self) -> Option<Arc<TextureReadback>> {
        self.current_readback.take()
    }
}

impl Default for TextureReadbackResource {
    fn default() -> Self {
        Self::new()
    }
}
