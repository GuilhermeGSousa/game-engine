use app::{plugins::Plugin, runner::ScheduleRunnerPlugin};
use ecs::{system::schedule::UpdateGroup, IntoSystemConfig};
use render::{device::RenderDevice, systems::render::finish_render};

use crate::{
    readback::{print_terminal_frame, TerminalRenderState},
    terminal_size::get_terminal_size,
};

pub struct TerminalRendererPlugin;

impl Plugin for TerminalRendererPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_plugin(ScheduleRunnerPlugin());
        app.add_system(
            UpdateGroup::LateRender,
            print_terminal_frame.after(finish_render),
        );
    }

    fn finish(&self, app: &mut app::App) {
        let (cols, rows) = get_terminal_size();

        let state = {
            let device = app.get_resource::<RenderDevice>().expect(
                "RenderDevice not found — make sure RenderPlugin is registered before TerminalRendererPlugin",
            );
            TerminalRenderState::new(&*device, cols, rows)
        };

        app.insert_resource(state);

        // Hide cursor and clear screen
        print!("\x1b[?25l\x1b[2J");
        use std::io::Write;
        std::io::stdout().flush().ok();
    }
}
