use app::Plugin;
use render::MaterialPlugin;

use crate::material::DebugGizmoMaterial;

pub struct DebugGizmosPlugin;

impl Plugin for DebugGizmosPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_plugin(MaterialPlugin::<DebugGizmoMaterial>::new());
        
    }
}
