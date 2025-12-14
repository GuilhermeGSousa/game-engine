pub mod assets;
pub mod blend;
pub mod tasks;
pub mod time;
pub mod transform;
pub mod utils;

#[cfg(test)]
mod tests {
    use ecs::{component::Component, world::World};
    use glam::{Quat, Vec3};

    use crate::transform::Transform;

    #[derive(Component)]
    struct Player;

    #[derive(Component)]
    struct Health;

    #[test]
    fn test_add_transform() {
        let mut world = World::new();

        let e = world.spawn(Transform::from_translation_rotation(
            Vec3::ZERO,
            Quat::IDENTITY,
        ));

        world.insert_component(Player, e);
        world.insert_component(
            Transform::from_translation_rotation(Vec3::ZERO, Quat::IDENTITY),
            e,
        );
        world.insert_component(Health, e);
    }
}
