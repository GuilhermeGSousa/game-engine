use std::{collections::HashMap, ops::Deref};

use ecs::component::Component;
use essential::assets::{asset_store::AssetStore, handle::AssetHandle};

use crate::{
    clip::AnimationClip,
    graph::{AnimationGraph, AnimationNodeIndex},
    node::{AnimationClipNodeState, AnimationNodeState},
};

pub struct ActiveNodeState {
    pub weight: f32,
    pub(crate) node_state: Box<dyn AnimationNodeState>,
}

impl ActiveNodeState {
    pub(crate) fn update(&mut self, delta_time: f32, duration: f32) {
        self.node_state.update(delta_time, duration);
    }
}

#[derive(Component, Default)]
pub struct AnimationPlayer {
    active_animations: HashMap<AnimationNodeIndex, ActiveNodeState>,
}

impl AnimationPlayer {
    pub fn get_node_state(&self, node_index: &AnimationNodeIndex) -> Option<&ActiveNodeState> {
        self.active_animations.get(node_index)
    }

    pub fn get_node_state_mut(
        &mut self,
        node_index: &AnimationNodeIndex,
    ) -> Option<&mut ActiveNodeState> {
        self.active_animations.get_mut(node_index)
    }

    pub fn initialize_states(&mut self, animation_graph: &AnimationGraph) {
        self.active_animations.clear();
        for node_index in animation_graph.iter() {
            let Some(anim_node) = animation_graph.get_node(node_index) else {
                continue;
            };

            let node_state = anim_node.create_state();
            self.active_animations.insert(
                node_index,
                ActiveNodeState {
                    node_state,
                    weight: 1.0,
                },
            );
        }
    }

    pub fn play(
        &mut self,
        node_index: &AnimationNodeIndex,
    ) {
        if let Some(anim_clip_state) = self
            .active_animations
            .get_mut(node_index)
            .and_then(|node_state| {
                node_state
                    .node_state
                    .as_any_mut()
                    .downcast_mut::<AnimationClipNodeState>()
            })
        {
            anim_clip_state.play();
        }
    }

    pub(crate) fn update(&mut self, delta_time: f32, graph: &AnimationGraph, animation_clips: &AssetStore<AnimationClip>) {
        self.active_animations
            .iter_mut()
            .for_each(|(node_index, node_state)| {
                let Some(node) = graph.get_node(*node_index) else
                {
                    return;
                };
                
                let Some(clip) = node.animation_clip().and_then(|clip_handle| animation_clips.get(clip_handle))
                else {
                    return;
                };

                node_state.update(delta_time, clip.duration());
            });
    }

    pub fn set_node_weight(&mut self, node_index: &AnimationNodeIndex, weight: f32)
    {
        if let Some(active_anim) = self.active_animations.get_mut(node_index){
            active_anim.weight = weight;
        }
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
