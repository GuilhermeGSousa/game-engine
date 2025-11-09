use std::ops::Deref;

use ecs::component::Component;
use essential::assets::handle::AssetHandle;

use crate::clip::AnimationClip;

pub struct ActiveAnimation {
    time: f32,
}

impl Default for ActiveAnimation {
    fn default() -> Self {
        Self { time: 0.0 }
    }
}

#[derive(Component, Default)]
pub struct AnimationPlayer {
    active_animation: ActiveAnimation,
}

impl AnimationPlayer {}

#[derive(Component)]
pub struct AnimationHandleComponent {
    pub handle: AssetHandle<AnimationClip>,
}

impl Deref for AnimationHandleComponent {
    type Target = AssetHandle<AnimationClip>;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
