use std::ops::Deref;

use ecs::resource::Resource;
use wgpu::{CommandEncoder, CommandEncoderDescriptor};

use crate::queue::RenderQueue;

#[derive(Resource)]
pub struct RenderDevice {
    pub(crate) device: wgpu::Device,
    pub(crate) encoder: Option<CommandEncoder>,
}

impl RenderDevice {
    pub fn command_encoder(&mut self) -> &mut CommandEncoder {
        self.encoder.get_or_insert_with(|| {
            self.device
                .create_command_encoder(&CommandEncoderDescriptor::default())
        })
    }

    pub fn finish(&mut self, queue: &RenderQueue) {
        if let Some(encoder) = self.encoder.take() {
            queue.submit(std::iter::once(encoder.finish()));
        }
    }
}

impl Deref for RenderDevice {
    type Target = wgpu::Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}
