use app::plugins::Plugin;
use ecs::{system::schedule::UpdateGroup, IntoSystemConfig};
use render::{device::RenderDevice, systems::render::finish_render};

use crate::{
    input::{poll_terminal_input, TerminalInput},
    readback::{print_terminal_frame, TerminalRenderState},
    runner::terminal_runner,
    terminal_size::get_terminal_size,
};

pub struct TerminalRendererPlugin;

impl Plugin for TerminalRendererPlugin {
    fn build(&self, app: &mut app::App) {
        app.set_runner(terminal_runner);
        app.add_system(UpdateGroup::Update, poll_terminal_input);
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
        app.insert_resource(TerminalInput::new());

        crossterm::terminal::enable_raw_mode().expect("Failed to enable raw mode");
        crossterm::execute!(
            std::io::stdout(),
            crossterm::event::EnableMouseCapture,
            crossterm::cursor::Hide,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        )
        .ok();
    }
}
