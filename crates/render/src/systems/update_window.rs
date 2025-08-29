use ecs::{
    query::Query,
    resource::{Res, ResMut},
};

use window::plugin::Window;

use crate::{
    components::camera::RenderCamera,
    device::RenderDevice,
    render_asset::{render_texture::RenderTexture, render_window::RenderWindow},
    resources::RenderContext,
};

pub(crate) fn update_window(
    mut window: ResMut<Window>,
    mut render_window: ResMut<RenderWindow>,
    mut context: ResMut<RenderContext>,
    device: Res<RenderDevice>,
    render_cameras: Query<(&mut RenderCamera,)>,
) {
    if window.should_resize() {
        let size = window.size();
        let surface = context.surface.clone();
        context.surface_config.width = size.0;
        context.surface_config.height = size.1;
        surface.configure(&device, &context.surface_config);

        for (mut render_camera,) in render_cameras.iter() {
            render_camera.depth_texture = RenderTexture::create_depth_texture(
                &device,
                &context.surface_config,
                "depth_texture",
            );
        }
        window.clear_resize();
    }

    if let Ok(output) = context.surface.get_current_texture() {
        render_window.set_swapchain_texture(output);
    }
}
