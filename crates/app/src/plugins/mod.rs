use core::time::Time;

use ecs::resource::ResMut;

use crate::update_group::UpdateGroup;
use crate::App;

pub trait Plugin {
    fn build(&self, app: &mut App);
}

pub struct TimePlugin;

fn update_time(mut time: ResMut<Time>) {
    time.update();
}

impl Plugin for TimePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::new());
        app.add_system(UpdateGroup::Update, update_time);
    }
}
