use app::Plugin;

use crate::material::DebugGizmoMaterial;

pub struct DebugGizmosPlugin;

impl Plugin for DebugGizmosPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_asset::<DebugGizmoMaterial>();
    }
}