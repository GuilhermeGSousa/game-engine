use std::collections::HashMap;

use ecs::{entity::Entity, resource::Resource};
use essential::transform::Transform;
use glam::{Quat, Vec3};
use rapier3d::{
    math::Vector,
    prelude::{
        CCDSolver, ColliderBuilder, ColliderHandle, ColliderSet, DefaultBroadPhase,
        ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet, NarrowPhase,
        QueryPipeline, RigidBodySet,
    },
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
    /// Maps every collider handed out by this state back to the ECS entity that owns it.
    ///
    /// Rapier's `user_data` field can't be used for this instead because `ecs::Entity` has no
    /// public constructor outside the ecs crate — entities are recorded here at creation time.
    pub(crate) collider_entities: HashMap<ColliderHandle, Entity>,
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
            collider_entities: HashMap::new(),
        }
    }

    fn track_collider(&mut self, handle: ColliderHandle, entity: Entity) -> Collider {
        self.collider_entities.insert(handle, entity);
        Collider(handle)
    }

    /// Removes a collider's entity mapping. Call when the owning [`Collider`] component is despawned.
    pub fn forget_collider(&mut self, collider: &Collider) {
        self.collider_entities.remove(&collider.0);
    }

    pub fn make_sphere(&mut self, entity: Entity, parent: &RigidBody, radius: f32) -> Collider {
        let col = ColliderBuilder::ball(radius).build();

        let handle = self
            .collider_set
            .insert_with_parent(col, **parent, &mut self.rigid_body_set);
        self.track_collider(handle, entity)
    }

    pub fn make_cuboid(
        &mut self,
        entity: Entity,
        width: f32,
        height: f32,
        length: f32,
        transform: &Transform,
        parent: Option<&RigidBody>,
    ) -> Collider {
        let pos = transform.translation;
        let col = ColliderBuilder::cuboid(width, height, length)
            .translation(Vector::new(pos.x, pos.y, pos.z))
            .build();

        let handle = match parent {
            Some(rb) => self
                .collider_set
                .insert_with_parent(col, **rb, &mut self.rigid_body_set),
            None => self.collider_set.insert(col),
        };
        self.track_collider(handle, entity)
    }

    /// Builds a Y-axis capsule collider (matches `Transform`'s Y-up convention), the standard
    /// shape for a kinematic character controller's capsule.
    pub fn make_capsule(
        &mut self,
        entity: Entity,
        radius: f32,
        half_height: f32,
        parent: &RigidBody,
    ) -> Collider {
        let col = ColliderBuilder::capsule_y(half_height, radius).build();

        let handle = self
            .collider_set
            .insert_with_parent(col, **parent, &mut self.rigid_body_set);
        self.track_collider(handle, entity)
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

impl Default for PhysicsState {
    fn default() -> Self {
        Self::new()
    }
}
