use app::{plugins::PluginsState, runner::AppExit, App};

pub(crate) fn terminal_runner(mut app: App) -> AppExit {
    if app.plugin_state() != PluginsState::Finished {
        while app.plugin_state() != PluginsState::Ready {}
    }

    if app.plugin_state() == PluginsState::Ready {
        app.finish_plugin_build();
    }

    let mut terminal = ratatui::init();

    loop {
        terminal
            .draw(|frame| {
                app.update();

                // Pass some wiget from app
                //f.render_widget(widget, area);
            })
            .unwrap();
    }
}
