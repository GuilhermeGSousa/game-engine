pub mod archetype;
pub mod bundle;
pub mod common;
pub mod component;
pub mod entity;
pub mod system;
pub mod table;
pub mod world;

#[cfg(test)]
mod tests {
    use crate::{component::Component, system::FnSystem, world};

    #[derive(Component)]
    struct Health;

    #[derive(Component)]
    struct Position;

    fn system((pos, health): (Position, Health)) {
        println!("System running");
    }

    #[test]
    fn spawn_entity() {
        let mut world = world::World::new();
        world.spawn((Health, Position));
        world.add_system(FnSystem::new(system));
    }
}
