use app::Plugin;
use essential::assets::asset_server::AssetServer;
use render::MaterialPlugin;

use crate::{components::sphere_added, material::DebugGizmoMaterial, shapes::GizmoShapes};

pub struct DebugGizmosPlugin;

impl Plugin for DebugGizmosPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_plugin(MaterialPlugin::<DebugGizmoMaterial>::new());

        app.add_system(ecs::system::schedule::UpdateGroup::Update, sphere_added);

        let asset_server = app
            .get_resource::<AssetServer>()
            .expect("AssetServer not found, make sure its plugin is registered");

        app.insert_resource(GizmoShapes {
            line: asset_server.add(GizmoShapes::make_line()),
            sphere: asset_server.add(GizmoShapes::make_unit_sphere()),
            cube: asset_server.add(GizmoShapes::make_unit_cube()),
        });
    }
}
