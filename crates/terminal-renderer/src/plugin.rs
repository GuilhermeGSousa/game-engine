use app::plugins::Plugin;
use ecs::{system::schedule::UpdateGroup, IntoSystemConfig};
use render::{device::RenderDevice, systems::render::finish_render};

use crate::{
    frame::TerminalFrames,
    input::{poll_terminal_input, TerminalInput},
    readback::{print_terminal_frame, TerminalRenderState},
    runner::terminal_runner,
    terminal::Terminal,
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

        app.insert_resource(TerminalFrames::default());
        app.insert_resource(Terminal::new(ratatui::init()));
    }

    fn finish(&self, app: &mut app::App) {
        let state = {
            let device = app.get_resource::<RenderDevice>().expect(
                "RenderDevice not found — make sure RenderPlugin is registered before TerminalRendererPlugin",
            );
            let terminal_size = app.get_resource::<Terminal>().unwrap().size().unwrap();
            TerminalRenderState::new(
                &*device,
                terminal_size.width as u32,
                terminal_size.height as u32,
            )
        };

        app.insert_resource(state);
        app.insert_resource(TerminalInput::new());
    }
}
