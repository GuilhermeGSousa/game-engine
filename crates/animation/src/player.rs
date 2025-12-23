use std::ops::Deref;

use ecs::component::Component;
use essential::assets::handle::AssetHandle;
use log::info;

use crate::{
    evaluation::AnimationGraphContext,
    graph::{AnimationGraph, AnimationGraphInstance, AnimationNodeIndex},
    node::{AnimationClipNodeInstance, AnimationNode, AnimationNodeInstance},
    state_machine::{AnimationFSMVariableType, AnimationStateMachineInstance},
};

pub struct ActiveNodeInstance {
    pub weight: f32,
    pub(crate) node_instance: Box<dyn AnimationNodeInstance>,
}

impl ActiveNodeInstance {
    pub(crate) fn update(
        &mut self,
        node: &Box<dyn AnimationNode>,
        delta_time: f32,
        context: &AnimationGraphContext<'_>,
    ) {
        self.node_instance.update(node, delta_time, context);
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

    pub(crate) fn initialize_graph(
        &mut self,
        animation_graph: AssetHandle<AnimationGraph>,
        context: &AnimationGraphContext,
    ) {
        self.graph_instance.initialize(animation_graph, context);
    }

    pub(crate) fn update(&mut self, delta_time: f32, context: &AnimationGraphContext) {
        self.graph_instance.update(delta_time, context);
    }

    pub(crate) fn graph_instance(&self) -> &AnimationGraphInstance {
        &self.graph_instance
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
