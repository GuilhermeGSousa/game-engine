use animation::{
    clip::AnimationClip,
    evaluation::AnimationGraphEvaluationContext,
    node::{AnimationNode, AnimationNodeState, NoneState},
};
use essential::{assets::handle::AssetHandle, transform::Transform};
use statig::prelude::{IntoStateMachine, State, StateMachine, Superstate};

pub(crate) struct AnimationFSMNode<T>
where
    T: IntoStateMachine<State = ()> + Send + Sync + 'static,
    T::State: State<T> + Send + Sync,
    for<'sub> T::Superstate<'sub>: Superstate<T>,
{
    fsm: StateMachine<T>,
}

impl<T> AnimationNode for AnimationFSMNode<T>
where
    T: IntoStateMachine<State = ()> + Send + Sync + 'static,
    T::State: State<T> + Send + Sync,
    for<'sub> T::Superstate<'sub>: Superstate<T>,
{
    fn create_state(&self) -> Box<dyn animation::node::AnimationNodeState> {
        Box::new(NoneState)
    }

    fn evaluate(&self, context: AnimationGraphEvaluationContext<'_>) -> Transform {
        todo!()
    }
}
