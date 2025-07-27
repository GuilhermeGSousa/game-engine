use ecs::resource::Resource;
use essential::transform::Transform;
use glam::{Quat, Vec3};
use rapier3d::prelude::{
    CCDSolver, ColliderBuilder, ColliderSet, DefaultBroadPhase, ImpulseJointSet,
    IntegrationParameters, IslandManager, MultibodyJointSet, NarrowPhase, QueryPipeline,
    RigidBodySet,
};

use crate::{collider::Collider, rigid_body::RigidBody};

#[derive(Resource)]
pub struct PhysicsState {
    pub(crate) rigid_body_set: RigidBodySet,
    pub(crate) collider_set: ColliderSet,
    pub(crate) integration_parameters: IntegrationParameters,
    pub(crate) island_manager: IslandManager,
    pub(crate) broad_phase: DefaultBroadPhase,
    pub(crate) narrow_phase: NarrowPhase,
    pub(crate) impulse_joint_set: ImpulseJointSet,
    pub(crate) multibody_joint_set: MultibodyJointSet,
    pub(crate) ccd_solver: CCDSolver,
    pub(crate) query_pipeline: QueryPipeline,
}

impl PhysicsState {
    pub fn new() -> Self {
        Self {
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            integration_parameters: IntegrationParameters::default(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
        }
    }

    pub fn make_sphere(&mut self, parent: &RigidBody, radius: f32) -> Collider {
        let col = ColliderBuilder::ball(radius).build();

        Collider(
            self.collider_set
                .insert_with_parent(col, **parent, &mut self.rigid_body_set),
        )
    }

    pub fn make_cuboid(
        &mut self,
        width: f32,
        height: f32,
        length: f32,
        parent: Option<&RigidBody>,
    ) -> Collider {
        let col = ColliderBuilder::cuboid(width, height, length).build();

        match parent {
            Some(rb) => Collider(self.collider_set.insert_with_parent(
                col,
                **rb,
                &mut self.rigid_body_set,
            )),
            None => Collider(self.collider_set.insert(col)),
        }
    }

    pub fn get_rigid_body(&self, rigid_body: &RigidBody) -> Transform {
        let rigid_body = &self.rigid_body_set[**rigid_body];
        let translation = rigid_body.translation();
        let translation = Vec3::new(translation.x, translation.y, translation.z);

        let rotation = rigid_body.rotation().coords;
        let rotation = Quat::from_array(rotation.into());

        Transform::from_translation_rotation(translation, rotation)
    }
}
