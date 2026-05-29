use app::{plugins::PluginsState, runner::AppExit, App};
use crossterm::event::{self, Event};

use crate::{frame::TerminalFrames, terminal::Terminal};

pub(crate) fn terminal_runner(mut app: App) -> AppExit {
    if app.plugin_state() != PluginsState::Finished {
        while app.plugin_state() != PluginsState::Ready {}
    }

    if app.plugin_state() == PluginsState::Ready {
        app.finish_plugin_build();
    }

    loop {
        app.update();

        let mut terminal = app.remove_resource::<Terminal>().expect(
            "Terminal resource does not exist, did you register the TerminalRender plugin?",
        );

        terminal.draw(|frame| {
            let terminal_frame = app.get_resource_mut::<TerminalFrames>().expect("TerminalFrames resource does not exist, did you register the TerminalRender plugin?");

            if let Some(data) = terminal_frame.pop_data()
            {
                frame.render_widget(data, frame.area());
            }
        }).unwrap();

        app.insert_resource(terminal);
        if matches!(event::read().unwrap(), Event::Key(_)) {
            break;
        }
    }

    ratatui::restore();
    AppExit::Success
}
