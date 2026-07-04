use ecs::entity::Entity;
use glam::Vec3;
use rapier3d::{
    geometry::Collider as RapierCollider,
    math::Point,
    pipeline::QueryFilter,
    prelude::{ColliderHandle, Ray},
};

use crate::physics_state::PhysicsState;

/// The result of a successful raycast against the physics world.
pub struct RaycastHit {
    /// The ECS entity that owns the collider that was hit, if the collider was created through
    /// [`PhysicsState`]'s constructors (which record the owning entity at creation time).
    pub entity: Option<Entity>,
    /// The world-space point where the ray hit the collider.
    pub point: Vec3,
    /// The outward-facing surface normal at the hit point.
    pub normal: Vec3,
    /// The distance from the ray origin to the hit point.
    pub distance: f32,
}

impl PhysicsState {
    /// Casts a ray into the physics world and returns the closest hit, if any.
    ///
    /// `direction` does not need to be normalized; `max_distance` is measured in the same units
    /// as `direction`'s length (i.e. it is scaled by `direction`'s magnitude), matching rapier's
    /// own `cast_ray_and_get_normal` contract.
    pub fn cast_ray(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        exclude: &[Entity],
    ) -> Option<RaycastHit> {
        let direction = direction.try_normalize()?;

        let ray = Ray::new(
            Point::new(origin.x, origin.y, origin.z),
            rapier3d::math::Vector::new(direction.x, direction.y, direction.z),
        );

        let exclude_handles: Vec<ColliderHandle> = self
            .collider_entities
            .iter()
            .filter(|(_, entity)| exclude.contains(entity))
            .map(|(handle, _)| *handle)
            .collect();

        let predicate = move |handle: ColliderHandle, _collider: &RapierCollider| {
            !exclude_handles.contains(&handle)
        };
        let filter = QueryFilter::new().predicate(&predicate);

        let (handle, intersection) = self.query_pipeline.cast_ray_and_get_normal(
            &self.rigid_body_set,
            &self.collider_set,
            &ray,
            max_distance,
            true,
            filter,
        )?;

        let point = ray.origin + ray.dir * intersection.time_of_impact;

        Some(RaycastHit {
            entity: self.collider_entities.get(&handle).copied(),
            point: Vec3::new(point.x, point.y, point.z),
            normal: Vec3::new(
                intersection.normal.x,
                intersection.normal.y,
                intersection.normal.z,
            ),
            distance: intersection.time_of_impact,
        })
    }
}
