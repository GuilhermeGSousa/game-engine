use std::ops::Deref;

use ecs::resource::Resource;

#[derive(Resource)]
pub struct RenderDevice {
    pub(crate) device: wgpu::Device,
}

impl Deref for RenderDevice {
    type Target = wgpu::Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}
