use std::any::Any;
use std::ops::Deref;
use std::{collections::HashMap, sync::Arc};

use crate::evaluation::EvaluatedNode;
use crate::graph::{AnimationGraph, AnimationGraphInstance};
use crate::target::AnimationTarget;
use crate::transition::blend_stack::BlendStack;
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
    Float(f32),
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

pub struct AnimationStateMachineTransitionDefinition<'a> {
    pub target_state: &'a str,
    pub trigger: AnimationFSMTrigger,
}

pub(crate) struct AnimationStateMachineTransition {
    next_state: StateId,
    trigger: AnimationFSMTrigger,
}

#[derive(Clone, Copy)]
pub struct StateId(usize);

impl Deref for StateId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<usize> for StateId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[derive(AsAny)]
pub struct AnimationStateMachine {
    initial_state: StateId,
    states: Vec<AnimationFSMState>,
    transitions: Vec<Vec<AnimationStateMachineTransition>>,
}

impl AnimationStateMachine {
    pub fn new(
        initial_state: &str,
        states_definition: Vec<AnimationFSMStateDefinition>,
        transitions_definition: HashMap<&str, Vec<AnimationStateMachineTransitionDefinition>>,
    ) -> Self {
        let mut name_to_index: HashMap<&str, StateId> = HashMap::new();
        let mut transitions = Vec::new();
        let states = states_definition
            .into_iter()
            .enumerate()
            .map(|(index, state_def)| {
                name_to_index.insert(state_def.name, index.into());
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
                    transitions[**from_index].push(AnimationStateMachineTransition {
                        next_state: *name_to_index
                            .get(transition_def.target_state)
                            .unwrap_or(&0.into()),
                        trigger: transition_def.trigger,
                    });
                });
            });

        Self {
            initial_state: *name_to_index.get(initial_state).unwrap_or(&0.into()),
            states,
            transitions,
        }
    }

    pub(crate) fn get_state(&self, state_index: StateId) -> Option<&AnimationFSMState> {
        self.states.get(*state_index)
    }

    pub(crate) fn get_state_transitions(
        &self,
        state_index: StateId,
    ) -> Option<&Vec<AnimationStateMachineTransition>> {
        self.transitions.get(*state_index)
    }
}

impl AnimationNode for AnimationStateMachine {
    fn create_instance(
        &self,
        creation_context: &AnimationGraphEvaluationContext,
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
}

#[derive(AsAny)]
pub(crate) struct AnimationStateMachineInstance {
    state_graph_instances: Vec<AnimationGraphInstance>,
    current_state: StateId,
    params: AnimationFSMParameters,
    blend_stack: BlendStack,
}

impl AnimationStateMachineInstance {
    pub(crate) fn new(initial_state: StateId, graph_instance: Vec<AnimationGraphInstance>) -> Self {
        Self {
            state_graph_instances: graph_instance,
            current_state: initial_state,
            params: HashMap::new(),
            blend_stack: BlendStack::new(),
        }
    }

    pub(crate) fn set_param(&mut self, param_name: String, param_value: AnimationFSMVariableType) {
        self.params.insert(param_name, param_value);
    }

    pub(crate) fn current_state(&self) -> StateId {
        self.current_state
    }
}

impl AnimationNodeInstance for AnimationStateMachineInstance {
    fn reset(&mut self) {}

    fn update(
        &mut self,
        node: &Box<dyn AnimationNode>,
        delta_time: f32,
        context: AnimationGraphEvaluationContext<'_>,
    ) {
        let Some(fsm) = node.as_any().downcast_ref::<AnimationStateMachine>() else {
            return;
        };

        let Some(transitions) = fsm.get_state_transitions(self.current_state) else {
            return;
        };

        // Right now, we do not support transitioning states is a transition is ongoing
        for transition in transitions {
            match &transition.trigger {
                AnimationFSMTrigger::Instant => {
                    self.state_graph_instances[*self.current_state].reset_nodes();
                    self.current_state = transition.next_state;
                    return;
                }
                AnimationFSMTrigger::Condition(cond_fn) => {
                    if cond_fn(&self.params) {
                        self.state_graph_instances[*self.current_state].reset_nodes();
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

        if let Some(graph_instance) = self.state_graph_instances.get_mut(*self.current_state) {
            graph_instance.update(
                delta_time,
                current_state_graph,
                context.animation_clips,
                context.animation_graphs,
            );
        };
    }

    fn evaluate(
        &self,
        node: &Box<dyn AnimationNode>,
        target: &AnimationTarget,
        _evaluated_inputs: &Vec<EvaluatedNode>,
        context: AnimationGraphEvaluationContext<'_>,
    ) -> Transform {
        let Some(fsm) = node.as_any().downcast_ref::<AnimationStateMachine>() else {
            return Transform::IDENTITY;
        };

        let Some(current_state_graph) = fsm
            .get_state(self.current_state())
            .and_then(|current_fsm_state| context.animation_graphs().get(&current_fsm_state.graph))
        else {
            return Transform::IDENTITY;
        };

        let Some(current_state_graph_instance) =
            self.state_graph_instances.get(*self.current_state())
        else {
            return Transform::IDENTITY;
        };

        let current_evaluated_state = current_state_graph.evaluate_target(
            target,
            current_state_graph_instance,
            context.animation_clips,
            context.animation_graphs,
        );

        current_evaluated_state
    }
}
