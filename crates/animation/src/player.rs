use std::ops::Deref;

use ecs::{component::Component, entity::Entity};
use essential::{assets::handle::AssetHandle, transform::Transform};
use log::info;
use uuid::Uuid;

use crate::{
    evaluation::{AnimationGraphContext, AnimationGraphEvaluator},
    graph::{AnimationGraph, AnimationGraphInstance, AnimationNodeIndex},
    node::{AnimationClipNodeInstance, AnimationNode, AnimationNodeInstance},
    pose::{Pose, PoseLayout},
    state_machine::{AnimationFSMVariableType, AnimationStateMachineInstance},
};

pub struct ActiveNodeInstance {
    pub weight: f32,
    pub(crate) node_instance: Box<dyn AnimationNodeInstance>,
}

impl ActiveNodeInstance {
    pub(crate) fn update(
        &mut self,
        node: &dyn AnimationNode,
        delta_time: f32,
        context: &AnimationGraphContext<'_>,
    ) {
        self.node_instance.update(node, delta_time, context);
    }
}

#[derive(Component, Default)]
pub struct AnimationPlayer {
    graph_instance: AnimationGraphInstance,
    layout: PoseLayout,
    current_pose: Pose,
    evaluator: AnimationGraphEvaluator,
    /// Entity holding the `SkeletonComponent` this player drives (bones are read from there
    /// rather than duplicated on the player).
    skeleton_entity: Option<Entity>,
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

    /// Installs the animation-specific layout this player drives and sizes the pose buffer.
    pub(crate) fn set_layout(&mut self, target_ids: Vec<Option<Uuid>>, bind_pose: Vec<Transform>) {
        self.layout = PoseLayout::new(target_ids, bind_pose);
        self.layout.seed(&mut self.current_pose);
    }

    /// Records the entity whose `SkeletonComponent` lists this player's bones.
    pub(crate) fn set_skeleton_entity(&mut self, entity: Entity) {
        self.skeleton_entity = Some(entity);
    }

    pub fn skeleton_entity(&self) -> Option<Entity> {
        self.skeleton_entity
    }

    /// Evaluates the graph once into `current_pose` for the whole skeleton.
    pub(crate) fn evaluate(&mut self, context: &AnimationGraphContext) {
        if self.layout.is_empty() {
            return;
        }

        // Disjoint borrows so the graph can read the layout while writing the pose using the
        // player-owned evaluator (whose pools persist across frames).
        let Self {
            graph_instance,
            layout,
            current_pose,
            evaluator,
            ..
        } = self;

        layout.seed(current_pose);
        graph_instance.evaluate(layout, context, evaluator, current_pose);
    }

    pub fn layout(&self) -> &PoseLayout {
        &self.layout
    }

    pub fn current_pose(&self) -> &Pose {
        &self.current_pose
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
