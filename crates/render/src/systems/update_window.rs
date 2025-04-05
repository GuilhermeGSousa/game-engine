use ecs::resource::ResMut;

use window::plugin::Window;

use crate::resources::RenderContext;

pub(crate) fn update_window(mut window: ResMut<Window>, mut context: ResMut<RenderContext>) {
    if window.should_resize() {
        let size = window.size();
        let surface = context.surface.clone();
        context.surface_config.width = size.0;
        context.surface_config.height = size.1;
        surface.configure(&context.device, &context.surface_config);
        window.clear_resize();
    }
}
