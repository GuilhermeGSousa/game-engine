use essential::assets::asset_server::{handle_asset_load_events, AssetServer};
use essential::time::Time;

use ecs::resource::ResMut;
use essential::transform::systems::propagate_global_transforms;

use crate::update_group::UpdateGroup;
use crate::App;

#[derive(PartialEq, Eq, Debug, Clone, Copy, PartialOrd, Ord)]
pub enum PluginsState {
    Building,
    Ready,
    Finished,
}

pub trait Plugin {
    fn build(&self, app: &mut App);

    fn ready(&self, _app: &App) -> bool {
        true
    }

    fn finish(&self, _app: &mut App) {}
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

pub struct TransformPlugin;

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(UpdateGroup::LateUpdate, propagate_global_transforms);
    }
}
