use std::{collections::HashMap, ops::Deref};

use ecs::component::Component;
use essential::assets::handle::AssetHandle;

use crate::{clip::AnimationClip, graph::{AnimationGraph, AnimationNodeIndex}};

pub struct ActiveAnimation {
    time: f32,
    is_paused: bool,
    play_rate: f32,
    animation_clip: AssetHandle<AnimationClip>,
}

impl ActiveAnimation {
    pub fn update(&mut self, delta_time: f32, duration: f32) {
        if self.is_paused {
            return;
        }

        self.time += delta_time * self.play_rate;

        if self.time > duration {
            self.time = 0.0;
        }
    }

    pub fn reset(&mut self, clip: AssetHandle<AnimationClip>)
    {
        self.time = 0.0;
        self.play_rate = 1.0;
        self.animation_clip = clip;
        self.is_paused = false;
    }

    pub fn current_time(&self) -> f32 {
        self.time
    }

    pub fn current_animation(&self) -> &AssetHandle<AnimationClip>
    {
        &self.animation_clip
    }
}

#[derive(Component, Default)]
pub struct AnimationPlayer {
    active_animations: HashMap<AnimationNodeIndex, ActiveAnimation>,
}

impl AnimationPlayer {
    pub fn get_active_animation(
        &self,
        node_index: &AnimationNodeIndex,
    ) -> Option<&ActiveAnimation> {
        self.active_animations.get(node_index)
    }

    pub fn get_active_animation_mut(
        &mut self,
        node_index: &AnimationNodeIndex,
    ) -> Option<&mut ActiveAnimation> {
        self.active_animations.get_mut(node_index)
    }

    pub fn start(&mut self, node_index: &AnimationNodeIndex, clip: AssetHandle<AnimationClip>) -> &mut ActiveAnimation {
        match self.active_animations.entry(*node_index) 
        {
            std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                let active_anim = occupied_entry.into_mut();
                active_anim.reset(clip);
                active_anim
            },
            std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(ActiveAnimation { time: 0.0, is_paused: false, play_rate: 1.0, animation_clip: clip })
            },
        }
    }

    pub fn active_animations(&self) -> &HashMap<AnimationNodeIndex, ActiveAnimation> {
        &self.active_animations
    }

    pub fn active_animations_mut(&mut self) -> &mut HashMap<AnimationNodeIndex, ActiveAnimation> {
        &mut self.active_animations
    }
}

#[derive(Component)]
pub struct AnimationHandleComponent {
    pub handle: AssetHandle<AnimationGraph>,
}

impl Deref for AnimationHandleComponent {
    type Target = AssetHandle<AnimationGraph>;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
