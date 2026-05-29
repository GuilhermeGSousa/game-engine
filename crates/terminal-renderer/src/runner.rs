use app::{plugins::PluginsState, runner::AppExit, App};
use crossterm::event::{self};
use ratatui::{layout::{Constraint, Layout}, style::Stylize, text::Span};
use ratatui::text::Line as TextLine;

use crate::{TerminalInput, frame::TerminalFrame, terminal::TerminalContext};

pub(crate) fn terminal_runner(mut app: App) -> AppExit {
    if app.plugin_state() != PluginsState::Finished {
        while app.plugin_state() != PluginsState::Ready {}
    }

    if app.plugin_state() == PluginsState::Ready {
        app.finish_plugin_build();
    }

    loop {
        app.update();

        let mut terminal = app.remove_resource::<TerminalContext>().expect(
            "Terminal resource does not exist, did you register the TerminalRender plugin?",
        );

        terminal.draw(|frame| {
            let terminal_frame = 
                app
                    .get_resource_mut::<TerminalFrame>()
                    .expect("TerminalFrames resource does not exist, did you register the TerminalRender plugin?");

            // All this needs to be a user system
            if let Some(data) = terminal_frame.current_frame()
            {
                let vertical = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).spacing(1);
                let horizontal = Layout::horizontal([Constraint::Percentage(100)]).spacing(1);
                let [top, main] = frame.area().layout(&vertical);
                let [area] = main.layout(&horizontal);

                
                let title = TextLine::from_iter([
                    Span::from("Canvas Widget").bold(),
                    Span::from(" (Press 'q' to quit)"),
                ]);

                frame.render_widget(title.centered(), top);
                frame.render_widget(data, area);
            }
        }).unwrap();

        app.insert_resource(terminal);

        if app
            .get_resource::<TerminalInput>()
            .expect("TerminalInput resource does not exist, did you register the TerminalRender plugin?")
            .is_key_active(event::KeyCode::Esc)
            {
                break;
            }
    }
    
    ratatui::restore();
    AppExit::Success
}
