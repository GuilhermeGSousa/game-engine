use std::ops::Deref;

use ecs::resource::Resource;

#[derive(Resource)]
pub struct RenderQueue {
    pub(crate) queue: wgpu::Queue,
}

impl Deref for RenderQueue {
    type Target = wgpu::Queue;

    fn deref(&self) -> &Self::Target {
        &self.queue
    }
}
