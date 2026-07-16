use app::Plugin;
use ecs::system::schedule::UpdateGroup;

use crate::camera::{CameraSettings, move_camera_pivot, update_entity_follow};

pub mod camera;
pub mod movement;
pub mod player;

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut app::App) {
        app.insert_resource(CameraSettings::default());

        app.add_system(UpdateGroup::Update, move_camera_pivot)
            .add_system(UpdateGroup::Update, update_entity_follow);
    }
}
