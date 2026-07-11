use game_engine::{
    ecs::CommandQueue, essential::transform::Transform, jolt_physics::collider::Collider,
    world_grid::WorldGrid,
};
use glam::Vec3;

pub(crate) fn spawn_scene(mut cmd: CommandQueue) {
    cmd.spawn(WorldGrid::default());

    // Static ground collider, sunk by its half-height so the top surface is
    // flush with the world grid's y = 0 plane.
    cmd.spawn((
        Collider::cuboid(200.0, 1.0, 200.0),
        Transform::from_translation(Vec3::Y * -1.0),
    ));
}
