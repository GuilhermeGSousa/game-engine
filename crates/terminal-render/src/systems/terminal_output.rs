use crate::{readback::TextureReadback, resource::TerminalCamera, AsciiConverter, TerminalDevice};
use ecs::{
    query::{query_filter::With, Query},
    resource::{Res, ResMut},
};
use render::{
    components::{camera::RenderCamera, render_entity::RenderEntity},
    device::RenderDevice,
    queue::RenderQueue,
};

pub fn terminal_output_system(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut terminal: ResMut<TerminalDevice>,
    converter: Res<AsciiConverter>,
    terminal_cameras: Query<(&RenderEntity,), With<TerminalCamera>>,
    render_cameras: Query<(&RenderCamera,)>,
) {
    terminal.update_size();
    let term_width = terminal.width();
    let term_height = terminal.height();

    let Some((render_entity,)) = terminal_cameras.iter().next() else {
        return;
    };
    let Some((render_camera,)) = render_cameras.get_entity(**render_entity) else {
        return;
    };
    let Some(render_target) = &render_camera.render_target else {
        return;
    };

    let texture = render_target.texture();

    let readback = match TextureReadback::new(&device, texture) {
        Ok(r) => r,
        Err(e) => {
            log::error!("terminal_output: readback init failed: {}", e);
            return;
        }
    };

    let pixel_data = match readback.read_texture(&device, &queue, texture) {
        Ok(d) => d,
        Err(e) => {
            log::error!("terminal_output: readback read failed: {}", e);
            return;
        }
    };

    let tex_width = readback.width() as usize;
    let tex_height = readback.height() as usize;
    if tex_width == 0 || tex_height == 0 {
        return;
    }

    let screen_grid = converter.pixels_to_screen(&pixel_data, tex_width, tex_height);
    terminal.render_frame(&screen_grid, tex_width, tex_height, term_width, term_height);
}
