use game_engine::{
    ecs::CommandQueue, essential::transform::Transform, jolt_physics::collider::Collider,
    world_grid::WorldGrid,
};
use glam::Vec3;

pub(crate) fn spawn_scene(mut cmd: CommandQueue) {
    cmd.spawn(WorldGrid::default());

    // Static ground collider: 200 x 1 x 200 half-extents centred at the
    // origin, so its top surface is at y = 1.
    cmd.spawn((
        Collider::cuboid(200.0, 1.0, 200.0),
        Transform::from_translation(Vec3::ZERO),
    ));
}
