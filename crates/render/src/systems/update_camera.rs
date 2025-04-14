use essential::transform::Transform;

use ecs::{query::Query, resource::ResMut};

use crate::{components::camera::Camera, resources::RenderContext};

pub(crate) fn update_camera(
    cameras: Query<(&Camera, &Transform)>,
    mut context: ResMut<RenderContext>,
) {
    if let Some((camera, transform)) = cameras.iter().next() {
        context.camera_uniform.update_view_proj(camera, transform);
        context.queue.write_buffer(
            &context.camera_buffer,
            0,
            bytemuck::cast_slice(&[context.camera_uniform]),
        );
    }
}
