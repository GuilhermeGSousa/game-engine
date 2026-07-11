use game_engine::{
    ecs::{CommandQueue, Res, ResMut},
    essential::{assets::asset_server::AssetServer, transform::Transform},
    jolt_physics::{physics_state::PhysicsState, rigid_body::RigidBody},
    world_grid::WorldGrid,
};
use glam::Vec3;

pub(crate) fn spawn_scene(
    mut cmd: CommandQueue,
    asset_server: Res<AssetServer>,
    mut physics: ResMut<PhysicsState>,
) {
    cmd.spawn(WorldGrid::default());

    let _ = physics.make_cuboid(
        200.0,
        1.0,
        200.0,
        &Transform::from_translation(Vec3::ZERO),
        None,
    );
}
