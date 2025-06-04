use ecs::resource::{Res, ResMut};
use window::{plugin::Window, winit_events::WinitEvents};

use crate::resources::UIRenderer;

pub(crate) fn handle_window_events(
    window_events: Res<WinitEvents>,
    mut ui_renderer: ResMut<UIRenderer>,
    window: Res<Window>,
) {
    for event in window_events.winit_events.iter() {
        let _ = ui_renderer
            .state
            .on_window_event(&window.window_handle, event);
    }
}
