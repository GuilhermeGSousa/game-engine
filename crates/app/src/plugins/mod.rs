use essential::assets::asset_server::{handle_asset_load_events, AssetServer};
use essential::time::Time;

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

pub struct AssetManagerPlugin;

impl Plugin for AssetManagerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AssetServer::new());
        app.add_system(UpdateGroup::Update, handle_asset_load_events);
    }
}
