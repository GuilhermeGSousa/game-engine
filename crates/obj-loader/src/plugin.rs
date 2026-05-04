use app::plugins::Plugin;
use ecs::system::schedule::UpdateGroup;

use crate::{
    mtl_loader::MTLMaterial,
    obj_loader::{OBJAsset, spawn_obj_component},
};

pub struct OBJPlugin;

impl Plugin for OBJPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_asset::<OBJAsset>()
            .register_asset::<MTLMaterial>();
        app.add_system(UpdateGroup::Update, spawn_obj_component);
    }
}
