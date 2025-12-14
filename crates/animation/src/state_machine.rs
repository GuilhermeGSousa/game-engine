use std::any::Any;
use std::{collections::HashMap, sync::Arc};

use crate::{
    clip::AnimationClip,
    evaluation::AnimationGraphEvaluationContext,
    node::{AnimationNode, AnimationNodeState},
};
use essential::{assets::handle::AssetHandle, transform::Transform, utils::AsAny};

pub struct AnimationFSMStateDefinition<'a> {
    pub name: &'a str,
    pub clip: AssetHandle<AnimationClip>,
}

pub(crate) struct AnimationFSMState {
    clip: AssetHandle<AnimationClip>,
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
            .iter()
            .enumerate()
            .map(|(index, state_def)| {
                name_to_index.insert(state_def.name, index);
                transitions.push(Vec::new());
                AnimationFSMState {
                    clip: state_def.clip.clone(),
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
    current_state: usize,
    time: f32,
    params: AnimationFSMParameters,
}

impl AnimationFSMNodeState {
    pub(crate) fn new(initial_state: usize) -> Self {
        Self {
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
    fn reset(&mut self) {}

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

        let Some(current_state_data) = fsm.get_current_state(self.current_state) else {
            return;
        };

        let Some(clip) = context.animation_clips.get(&current_state_data.clip) else {
            return;
        };

        self.time += context.delta_time;

        if self.time > clip.duration() {
            self.time = 0.0;
        }
        for transition in transitions {
            match &transition.trigger {
                AnimationFSMTrigger::Instant => {
                    self.current_state = transition.next_state;
                    return;
                }
                AnimationFSMTrigger::Condition(cond_fn) => {
                    if cond_fn(&self.params) {
                        self.current_state = transition.next_state;
                        return;
                    }
                }
            }
        }
    }
}

impl AnimationNode for AnimationFSM {
    fn create_state(&self) -> Box<dyn AnimationNodeState> {
        Box::new(AnimationFSMNodeState::new(self.initial_state))
    }

    fn evaluate(
        &self,
        context: AnimationGraphEvaluationContext<'_>,
    ) -> essential::transform::Transform {
        let Some(node_state) = context
            .current_node_state()
            .as_any()
            .downcast_ref::<AnimationFSMNodeState>()
        else {
            return Transform::IDENTITY;
        };

        let Some(current_state) = self.get_current_state(node_state.current_state()) else {
            return Transform::IDENTITY;
        };

        let Some(animation_clip) = context.animation_clips().get(&current_state.clip) else {
            return Transform::IDENTITY;
        };

        let Some(animation_channels) = animation_clip.get_channels(&context.target_id()) else {
            return Transform::IDENTITY;
        };

        let mut transform = Transform::IDENTITY;
        for animation_channel in animation_channels {
            animation_channel.sample_transform(node_state.time, &mut transform);
        }
        transform
    }
}
