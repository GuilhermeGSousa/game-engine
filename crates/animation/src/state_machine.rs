use std::any::Any;
use std::{collections::HashMap, sync::Arc};

use crate::evaluation::AnimationGraphCreationContext;
use crate::graph::{AnimationGraph, AnimationGraphInstance};
use crate::target::AnimationTarget;
use crate::{
    evaluation::AnimationGraphEvaluationContext,
    node::{AnimationNode, AnimationNodeInstance},
};
use essential::{assets::handle::AssetHandle, transform::Transform, utils::AsAny};

pub struct AnimationFSMStateDefinition<'a> {
    pub name: &'a str,
    pub graph: AssetHandle<AnimationGraph>,
}

pub(crate) struct AnimationFSMState {
    graph: AssetHandle<AnimationGraph>,
}

pub enum AnimationFSMVariableType {
    Bool(bool),
    Int(u32),
}

pub type AnimationFSMParameters = HashMap<String, AnimationFSMVariableType>;

pub enum AnimationFSMTrigger {
    Instant,
    Condition(Arc<dyn Fn(&AnimationFSMParameters) -> bool + Send + Sync>),
}

impl AnimationFSMTrigger {
    pub fn from_condition<F>(condition: F) -> Self
    where
        F: Fn(&AnimationFSMParameters) -> bool + Send + Sync + 'static,
    {
        Self::Condition(Arc::new(condition))
    }
}

pub struct AnimationFSMTransitionDefinition<'a> {
    pub target_state: &'a str,
    pub trigger: AnimationFSMTrigger,
}

pub(crate) struct AnimationFSMTransition {
    next_state: usize,
    trigger: AnimationFSMTrigger,
}

#[derive(AsAny)]
pub struct AnimationStateMachine {
    initial_state: usize,
    states: Vec<AnimationFSMState>,
    transitions: Vec<Vec<AnimationFSMTransition>>,
}

impl AnimationStateMachine {
    pub fn new(
        initial_state: &str,
        states_definition: Vec<AnimationFSMStateDefinition>,
        transitions_definition: HashMap<&str, Vec<AnimationFSMTransitionDefinition>>,
    ) -> Self {
        let mut name_to_index = HashMap::new();
        let mut transitions = Vec::new();
        let states = states_definition
            .into_iter()
            .enumerate()
            .map(|(index, state_def)| {
                name_to_index.insert(state_def.name, index);
                transitions.push(Vec::new());
                AnimationFSMState {
                    graph: state_def.graph,
                }
            })
            .collect();

        transitions_definition
            .into_iter()
            .for_each(|(from, transition_defs)| {
                let Some(from_index) = name_to_index.get(from) else {
                    return;
                };

                transition_defs.into_iter().for_each(|transition_def| {
                    transitions[*from_index].push(AnimationFSMTransition {
                        next_state: *name_to_index.get(transition_def.target_state).unwrap_or(&0),
                        trigger: transition_def.trigger,
                    });
                });
            });

        Self {
            initial_state: *name_to_index.get(initial_state).unwrap_or(&0),
            states,
            transitions,
        }
    }

    pub(crate) fn get_state(&self, state_index: usize) -> Option<&AnimationFSMState> {
        self.states.get(state_index)
    }

    pub(crate) fn get_state_transitions(
        &self,
        state_index: usize,
    ) -> Option<&Vec<AnimationFSMTransition>> {
        self.transitions.get(state_index)
    }
}

#[derive(AsAny)]
pub(crate) struct AnimationStateMachineInstance {
    state_graph_instances: Vec<AnimationGraphInstance>,
    current_state: usize,
    params: AnimationFSMParameters,
}

impl AnimationNode for AnimationStateMachine {
    fn create_instance(
        &self,
        creation_context: &AnimationGraphCreationContext,
    ) -> Box<dyn AnimationNodeInstance> {
        let mut instanced_internal_graphs = Vec::new();
        for fsm_state in &self.states {
            let Some(fsm_state_graph) = creation_context.animation_graphs.get(&fsm_state.graph)
            else {
                continue;
            };

            let mut instanced_internal_graph = AnimationGraphInstance::default();
            instanced_internal_graph.initialize(fsm_state_graph, creation_context);
            instanced_internal_graphs.push(instanced_internal_graph);
        }

        Box::new(AnimationStateMachineInstance::new(
            self.initial_state,
            instanced_internal_graphs,
        ))
    }

    fn evaluate(
        &self,
        target: &AnimationTarget,
        context: AnimationGraphEvaluationContext<'_>,
    ) -> Transform {
        let Some(fsm_instance) = context
            .current_node_state()
            .as_any()
            .downcast_ref::<AnimationStateMachineInstance>()
        else {
            return Transform::IDENTITY;
        };

        let Some(current_state_graph_instance) = fsm_instance
            .state_graph_instances
            .get(fsm_instance.current_state)
        else {
            return Transform::IDENTITY;
        };

        let Some(current_fsm_state) = self.get_state(fsm_instance.current_state()) else {
            return Transform::IDENTITY;
        };

        let Some(animation_graph) = context.animation_graphs().get(&current_fsm_state.graph) else {
            return Transform::IDENTITY;
        };

        animation_graph.evaluate_target(
            target,
            current_state_graph_instance,
            context.animation_clips,
            context.animation_graphs,
        )
    }
}

impl AnimationStateMachineInstance {
    pub(crate) fn new(initial_state: usize, graph_instance: Vec<AnimationGraphInstance>) -> Self {
        Self {
            state_graph_instances: graph_instance,
            current_state: initial_state,
            params: HashMap::new(),
        }
    }

    pub(crate) fn set_param(&mut self, param_name: String, param_value: AnimationFSMVariableType) {
        self.params.insert(param_name, param_value);
    }

    pub(crate) fn current_state(&self) -> usize {
        self.current_state
    }
}

impl AnimationNodeInstance for AnimationStateMachineInstance {
    fn reset(&mut self) {}

    fn update(&mut self, context: crate::evaluation::AnimationGraphUpdateContext<'_>) {
        let Some(fsm) = context
            .animation_node
            .as_any()
            .downcast_ref::<AnimationStateMachine>()
        else {
            return;
        };

        let Some(transitions) = fsm.get_state_transitions(self.current_state) else {
            return;
        };

        for transition in transitions {
            match &transition.trigger {
                AnimationFSMTrigger::Instant => {
                    self.state_graph_instances[self.current_state].reset_nodes();
                    self.current_state = transition.next_state;
                    return;
                }
                AnimationFSMTrigger::Condition(cond_fn) => {
                    if cond_fn(&self.params) {
                        self.state_graph_instances[self.current_state].reset_nodes();
                        self.current_state = transition.next_state;
                        return;
                    }
                }
            }
        }

        let Some(current_state_graph) = fsm
            .get_state(self.current_state)
            .and_then(|current_state_data| context.animation_graphs.get(&current_state_data.graph))
        else {
            return;
        };

        if let Some(graph_instance) = self.state_graph_instances.get_mut(self.current_state) {
            graph_instance.update(
                context.delta_time,
                current_state_graph,
                context.animation_clips,
                context.animation_graphs,
            )
        };
    }
}
