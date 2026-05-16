use crate::{AsciiConverter, TerminalDevice};
use app::{update_group::UpdateGroup, App};
use app::plugins::Plugin;

pub struct TerminalRenderPlugin;

impl Plugin for TerminalRenderPlugin {
    fn build(&self, app: &mut App) {
        match TerminalDevice::new() {
            Ok(terminal) => {
                app.insert_resource(terminal);
            }
            Err(e) => {
                log::error!("Failed to initialize terminal device: {}", e);
                return;
            }
        }

        app.insert_resource(AsciiConverter::new());
    }

    fn finish(&self, app: &mut App) {
        app.add_system(UpdateGroup::LateRender, crate::systems::terminal_output_system);
    }
}
