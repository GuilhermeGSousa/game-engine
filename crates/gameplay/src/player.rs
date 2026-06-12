use ecs::{CommandQueue, Component, component::bundle::ComponentBundle};
use essential::transform::Transform;
use glam::{Quat, Vec3};
use render::components::camera::Camera;

#[derive(Component)]
pub struct Player;

pub fn spawn_first_person_player<T: ComponentBundle + 'static>(
    cmd: &mut CommandQueue,
    pos: Vec3,
    extra_components: T,
) {
    let camera = Camera::default();

    cmd.spawn((
        Player,
        Transform::from_translation_rotation(pos, Quat::IDENTITY),
        extra_components,
    ))
    .add_child((
        camera,
        Transform::from_translation_rotation(Vec3::ZERO, Quat::IDENTITY),
    ));
}
