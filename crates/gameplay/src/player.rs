use ecs::{CommandQueue, Component};
use essential::transform::Transform;
use glam::{Quat, Vec3};
use render::components::camera::Camera;

#[derive(Component)]
pub struct Player;

pub fn spawn_first_person_player(cmd: &mut CommandQueue, pos: Vec3) {
    let camera = Camera::default();

    cmd.spawn((
        Player,
        Transform::from_translation_rotation(Vec3::ZERO, Quat::IDENTITY),
    ))
    .add_child((
        camera,
        Transform::from_translation_rotation(pos, Quat::IDENTITY),
    ));
}
