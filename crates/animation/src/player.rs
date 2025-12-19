use std::ops::Deref;

use ecs::component::Component;
use essential::assets::{asset_store::AssetStore, handle::AssetHandle};
use log::info;

use crate::{
    clip::AnimationClip,
    evaluation::{AnimationGraphCreationContext, AnimationGraphUpdateContext},
    graph::{AnimationGraph, AnimationGraphInstance, AnimationNodeIndex},
    node::{AnimationClipNodeInstance, AnimationNodeInstance},
    state_machine::{AnimationFSMVariableType, AnimationStateMachineInstance},
};

pub struct ActiveNodeInstance {
    pub weight: f32,
    pub(crate) node_instance: Box<dyn AnimationNodeInstance>,
}

impl ActiveNodeInstance {
    pub(crate) fn update(&mut self, context: AnimationGraphUpdateContext<'_>) {
        self.node_instance.update(context);
    }
}

#[derive(Component, Default)]
pub struct AnimationPlayer {
    graph_instance: AnimationGraphInstance,
}

impl AnimationPlayer {
    pub fn play(&mut self, node_index: &AnimationNodeIndex) {
        if let Some(anim_clip_instance) = self
            .graph_instance
            .get_instance_mut::<AnimationClipNodeInstance>(node_index)
        {
            anim_clip_instance.play();
        }
    }

    pub(crate) fn get_node_instance(
        &self,
        node_index: &AnimationNodeIndex,
    ) -> Option<&ActiveNodeInstance> {
        self.graph_instance.get_active_node_instance(node_index)
    }

    pub(crate) fn initialize_graph(
        &mut self,
        animation_graph: &AnimationGraph,
        creation_context: &AnimationGraphCreationContext,
    ) {
        self.graph_instance
            .initialize(animation_graph, creation_context);
    }

    pub(crate) fn update(
        &mut self,
        delta_time: f32,
        graph: &AnimationGraph,
        animation_clips: &AssetStore<AnimationClip>,
        animation_graphs: &AssetStore<AnimationGraph>,
    ) {
        self.graph_instance
            .update(delta_time, graph, animation_clips, animation_graphs);
    }

    pub fn set_node_weight(&mut self, node_index: &AnimationNodeIndex, weight: f32) {
        self.graph_instance.set_node_weight(node_index, weight);
    }

    pub fn set_fsm_param<T: Into<String>>(
        &mut self,
        node_index: &AnimationNodeIndex,
        param_name: T,
        param_value: AnimationFSMVariableType,
    ) {
        let Some(fsm_instance) = self
            .graph_instance
            .get_instance_mut::<AnimationStateMachineInstance>(node_index)
        else {
            info!("No animation node found when setting FSM parameters");
            return;
        };

        fsm_instance.set_param(param_name.into(), param_value);
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
