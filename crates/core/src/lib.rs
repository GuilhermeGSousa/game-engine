pub mod archetype;
pub mod bundle;
pub mod common;
pub mod component;
pub mod entity;
pub mod world;

#[cfg(test)]
mod tests {
    use crate::{component::Component, world};

    #[derive(Component)]
    struct Health;

    #[derive(Component)]
    struct Position;

    #[test]
    fn spawn_entity() {
        let mut world = world::World::new();
        world.spawn((Health, Position));
    }
}
