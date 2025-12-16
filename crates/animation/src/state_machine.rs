use std::any::Any;
use std::{collections::HashMap, sync::Arc};

use crate::evaluation::{
    AnimationGraphCreationContext, AnimationGraphEvaluator, AnimationGraphUpdateContext,
    EvaluatedNode,
};
use crate::graph::{AnimationGraph, AnimationNodeIndex};
use crate::player::ActiveNodeState;
use crate::target::AnimationTarget;
use crate::{
    evaluation::AnimationGraphEvaluationContext,
    node::{AnimationNode, AnimationNodeState},
};
use essential::{assets::handle::AssetHandle, transform::Transform, utils::AsAny};
use log::warn;

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
pub struct AnimationFSM {
    initial_state: usize,
    states: Vec<AnimationFSMState>,
    transitions: Vec<Vec<AnimationFSMTransition>>,
}

impl AnimationFSM {
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

    pub(crate) fn get_current_state(&self, state_index: usize) -> Option<&AnimationFSMState> {
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
pub(crate) struct AnimationFSMNodeState {
    graph_state: HashMap<AnimationNodeIndex, ActiveNodeState>,
    current_state: usize,
    time: f32,
    params: AnimationFSMParameters,
}

impl AnimationFSMNodeState {
    pub(crate) fn new(
        initial_state: usize,
        graph_state: HashMap<AnimationNodeIndex, ActiveNodeState>,
    ) -> Self {
        Self {
            graph_state,
            current_state: initial_state,
            time: 0.0,
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

impl AnimationNodeState for AnimationFSMNodeState {
    fn update(&mut self, context: crate::evaluation::AnimationGraphUpdateContext<'_>) {
        let Some(fsm) = context
            .animation_node
            .as_any()
            .downcast_ref::<AnimationFSM>()
        else {
            return;
        };

        let Some(transitions) = fsm.get_state_transitions(self.current_state) else {
            return;
        };

        for transition in transitions {
            match &transition.trigger {
                AnimationFSMTrigger::Instant => {
                    self.current_state = transition.next_state;
                    self.time = 0.0;
                    return;
                }
                AnimationFSMTrigger::Condition(cond_fn) => {
                    if cond_fn(&self.params) {
                        self.current_state = transition.next_state;
                        self.time = 0.0;
                        return;
                    }
                }
            }
        }

        let Some(current_state_data) = fsm.get_current_state(self.current_state) else {
            return;
        };

        let Some(graph) = context.animation_graphs.get(&current_state_data.graph) else {
            return;
        };

        self.graph_state
            .iter_mut()
            .for_each(|(node_idx, node_state)| {
                let Some(node) = graph.get_node(*node_idx) else {
                    return;
                };

                let context = AnimationGraphUpdateContext {
                    animation_node: node,
                    delta_time: context.delta_time,
                    animation_clips: context.animation_clips,
                    animation_graphs: context.animation_graphs,
                };

                node_state.update(context);
            });
    }
}

impl AnimationNode for AnimationFSM {
    fn create_state(
        &self,
        creation_context: &AnimationGraphCreationContext,
    ) -> Box<dyn AnimationNodeState> {
        let mut internal_graph_state = HashMap::new();
        for state in &self.states {
            let Some(state_graph) = creation_context.animation_graphs.get(&state.graph) else {
                continue;
            };

            for state_fsm_index in state_graph.iter() {
                let Some(state_fsm_node) = state_graph.get_node(state_fsm_index) else {
                    continue;
                };

                let node_animation_state = state_fsm_node.create_state(creation_context);
                internal_graph_state.insert(
                    state_fsm_index,
                    ActiveNodeState {
                        weight: 1.0,
                        node_state: node_animation_state,
                    },
                );
            }
        }
        Box::new(AnimationFSMNodeState::new(
            self.initial_state,
            internal_graph_state,
        ))
    }

    fn evaluate(
        &self,
        target: &AnimationTarget,
        context: AnimationGraphEvaluationContext<'_>,
    ) -> Transform {
        let Some(node_state) = context
            .current_node_state()
            .as_any()
            .downcast_ref::<AnimationFSMNodeState>()
        else {
            return Transform::IDENTITY;
        };

        let Some(current_fsm_state) = self.get_current_state(node_state.current_state()) else {
            return Transform::IDENTITY;
        };

        let Some(animation_graph) = context.animation_graphs().get(&current_fsm_state.graph) else {
            return Transform::IDENTITY;
        };

        let mut graph_evaluator = AnimationGraphEvaluator::new();

        for node_index in animation_graph.iter_post_order() {
            let Some(node) = animation_graph.get_node(node_index) else {
                continue;
            };

            let Some(node_state) = node_state.graph_state.get(&node_index) else {
                warn!(
                    "No node state found for node, make sure the animation player has been correctly initialized"
                );
                continue;
            };

            let evaluated_inputs = animation_graph
                .get_node_inputs(node_index)
                .map(|_| graph_evaluator.pop_evaluation())
                .filter_map(|transform| transform)
                .collect::<Vec<_>>();

            let context = AnimationGraphEvaluationContext {
                node_state,
                animation_clips: &context.animation_clips,
                animation_graphs: &context.animation_graphs,
                evaluated_inputs: &evaluated_inputs,
            };

            graph_evaluator.push_evaluation(EvaluatedNode {
                transform: node.evaluate(&target, context),
                weight: node_state.weight,
            });
        }

        graph_evaluator
            .pop_evaluation()
            .map(|evaluated_node| evaluated_node.transform)
            .unwrap_or(Transform::IDENTITY)
    }
}
