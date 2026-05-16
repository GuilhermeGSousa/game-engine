use crate::{AsciiConverter, TerminalDevice, TextureReadbackResource};
use anyhow::Result;
use app::{plugin::Plugin, App};
use ecs::app_data::AppData;

pub struct TerminalRenderPlugin;

impl Plugin for TerminalRenderPlugin {
    fn build(&self, app: &mut App) {
        // Initialize terminal device
        match TerminalDevice::new() {
            Ok(terminal) => {
                app.insert_resource(terminal);
            }
            Err(e) => {
                log::error!("Failed to initialize terminal device: {}", e);
                return;
            }
        }

        // Insert ASCII converter
        app.insert_resource(AsciiConverter::new());

        // Insert readback resource
        app.insert_resource(TextureReadbackResource::new());
    }

    fn ready(&self, app: &App) -> bool {
        // Wait for render plugin to be ready
        true
    }

    fn finish(&self, app: &mut App) {
        // Add the terminal output system in the LateRender phase
        app.add_system(ecs::app_data::UpdateGroup::LateRender, crate::systems::terminal_output_system);
    }
}
