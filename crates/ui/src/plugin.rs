use app::plugins::Plugin;
use render::resources::RenderContext;

use crate::{
    input::handle_window_events,
    render::{begin_ui_frame, end_ui_frame},
    resources::UIRenderer,
};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut app::App) {
        let render_context = app
            .get_resource::<RenderContext>()
            .expect("RenderContext resource not found");

        let window = app
            .get_resource::<window::plugin::Window>()
            .expect("Window resource not found");

        app.insert_resource(UIRenderer::new(
            &window.window_handle,
            &render_context.device,
            render_context.surface_config.format,
            None,
            1,
            true,
        ));

        app.add_system(app::update_group::UpdateGroup::Update, handle_window_events);
        app.add_system(app::update_group::UpdateGroup::Render, begin_ui_frame);
        app.add_system_first(app::update_group::UpdateGroup::LateRender, end_ui_frame);
    }
}
