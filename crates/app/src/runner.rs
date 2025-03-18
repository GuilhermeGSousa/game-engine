use crate::{plugin::Plugin, App};

pub enum AppExit {
    Success,
    Error(u8),
}

pub(crate) type RunnerFn = Box<dyn FnOnce(App) -> AppExit>;

pub fn run_once(_app: App) -> AppExit {
    AppExit::Success
}

pub struct ScheduleRunnerPlugin();

impl Plugin for ScheduleRunnerPlugin {
    fn build(&self, app: &mut App) {
        app.set_runner(move |mut app: App| loop {
            app.update();
        });
    }
}
