use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

pub mod bundle;

pub use ecs_macros::Component;

use crate::{entity::Entity, world::RestrictedWorld};

/// A unique identifier for a component type, based on its Rust [`TypeId`].
pub type ComponentId = TypeId;

/// Callback invoked when a component is added to or removed from an entity.
///
/// The callback receives a [`RestrictedWorld`] so it can safely react to the
/// lifecycle event (e.g. inserting or removing companion components).
pub type ComponentLifecycleCallback = for<'w> fn(RestrictedWorld<'w>, ComponentLifecycleContext);

/// Context passed to a [`ComponentLifecycleCallback`].
pub struct ComponentLifecycleContext {
    /// The entity whose component triggered the callback.
    pub entity: Entity,
}

/// Marker trait for data that can be attached to an [`Entity`](crate::entity::Entity).
///
/// Implement this trait (or derive it with `#[derive(Component)]`) for any type that
/// should live in the ECS storage.
///
/// # Lifecycle callbacks
/// Override [`on_add`](Component::on_add) or [`on_remove`](Component::on_remove) to
/// run logic automatically when the component is added to or removed from an entity.
///
/// # Example
/// ```
/// use ecs::component::Component;
///
/// #[derive(Component)]
/// struct Velocity {
///     x: f32,
///     y: f32,
/// }
/// ```
pub trait Component: Send + Sync + 'static {
    /// Returns the human-readable name of this component (usually the type name).
    fn name() -> &'static str;

    /// Optional callback invoked immediately after this component is added to an entity.
    fn on_add() -> Option<ComponentLifecycleCallback> {
        None
    }

    /// Optional callback invoked immediately before this component is removed from an entity.
    fn on_remove() -> Option<ComponentLifecycleCallback> {
        None
    }
}

// TODO: Implement add lifecycle
#[allow(dead_code)]
pub(crate) struct ComponentLifecycleCallbacks {
    pub(crate) on_add: Option<ComponentLifecycleCallback>,
    pub(crate) on_remove: Option<ComponentLifecycleCallback>,
}

impl ComponentLifecycleCallbacks {
    pub(crate) fn from_component<T: Component>() -> Self {
        Self {
            on_add: T::on_add(),
            on_remove: T::on_remove(),
        }
    }
}

/// A monotonically-increasing frame counter used for change detection.
///
/// Each component and resource stores the tick at which it was last added or
/// mutated.  Filters like [`Added`](crate::query::query_filter::Added) and
/// [`Changed`](crate::query::query_filter::Changed) compare the stored tick
/// against the world's current tick to decide whether to include an entity.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Tick(u32);

impl Tick {
    /// Creates a new `Tick` with the given counter value.
    pub fn new(tick: u32) -> Self {
        Self(tick)
    }

    /// Sets the tick value.
    pub fn set(&mut self, tick: u32) {
        self.0 = tick;
    }
}

impl Deref for Tick {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Tick {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
