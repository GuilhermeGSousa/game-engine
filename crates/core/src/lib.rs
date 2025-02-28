pub mod archetype;
pub mod bundle;
pub mod common;
pub mod component;
pub mod entity;
pub mod query;
pub mod resource;
pub mod schedule;
pub mod system;
pub mod system_metadata;
pub mod table;
pub mod world;

#[cfg(test)]
mod tests {

    use crate::{component::Component, schedule, world};

    #[derive(Component)]
    struct Health;

    #[derive(Component)]
    struct Position;

    fn system(pos: &mut Position, vel: &Health) {
        println!("{:?} {:?}", Position::name(), Health::name());
    }

    #[test]
    fn spawn_entity() {
        let schedule = schedule::Scheduler::default().add_system(system);

        let mut world = world::World::new();
        world.spawn((Health, Position));

        schedule.run(&mut world);
    }
}
