use ecs::{
    events::event_reader::EventReader,
    resource::{Res, ResMut},
};
use window::{plugin::Window, winit_events::WinitEvent};

use crate::resources::UIRenderer;

pub(crate) fn handle_window_events(
    window_events: EventReader<WinitEvent>,
    mut ui_renderer: ResMut<UIRenderer>,
    window: Res<Window>,
) {
    for event in window_events.read() {
        let _ = ui_renderer
            .state
            .on_window_event(&window.window_handle, event);
    }
}
