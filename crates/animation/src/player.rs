use std::ops::Deref;

use ecs::component::Component;
use essential::assets::handle::AssetHandle;

use crate::clip::AnimationClip;

pub struct ActiveAnimation {
    time: f32,
    duration: f32,
    is_paused: bool,
    play_rate: f32,
}

impl Default for ActiveAnimation {
    fn default() -> Self {
        Self {
            time: 0.0,
            duration: 0.0,
            is_paused: false,
            play_rate: 1.0,
        }
    }
}

impl ActiveAnimation {
    pub fn update(&mut self, delta_time: f32) {
        if self.is_paused {
            return;
        }

        self.time += delta_time * self.play_rate;

        if self.time > self.duration {
            self.time = 0.0;
        }
    }
}

#[derive(Component, Default)]
pub struct AnimationPlayer {
    active_animation: ActiveAnimation,
}

impl AnimationPlayer {
    pub fn update(&mut self, delta_time: f32) {
        self.active_animation.update(delta_time);
    }

    pub fn current_time(&self) -> f32 {
        self.active_animation.time
    }

    pub fn play(&mut self, clip: &AnimationClip) {
        self.active_animation.duration = clip.duration();
        self.active_animation.time = 0.0;
    }
}

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
