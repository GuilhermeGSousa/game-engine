//! Engine-native editor. Panels submit commands; project systems own I/O.
//!
//! The editor is read-only: it opens a project, spawns scenes into the world,
//! and inspects what is there. Importing is the `import` CLI's job.
pub mod actions;
pub mod content;
pub mod diagnostics;
pub mod dock;
pub mod fonts;
pub mod hierarchy;
pub mod inspector;
pub mod marks;
pub mod project;
pub mod scene;
pub mod selection;
pub mod shell;
pub mod viewport;
pub mod window_chrome;

use app::{App, Plugin};
use std::path::PathBuf;

pub struct EditorPlugin {
    pub project: Option<PathBuf>,
    /// Let the window manager draw the window frame instead of the editor.
    pub decorated: bool,
}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(selection::Selection::default());
        app.insert_resource(dock::PanelRegistry::default());
        app.register_plugin(fonts::FontsPlugin);
        app.register_plugin(dock::DockPlugin);
        app.register_plugin(actions::ActionsPlugin);
        app.insert_resource(project::EditorCommands::default());
        app.insert_resource(project::ProjectState::default());
        app.register_plugin(project::ProjectPlugin);
        app.register_plugin(scene::ScenePlugin);
        app.register_plugin(hierarchy::HierarchyPlugin);
        app.register_plugin(content::ContentPlugin);
        app.register_plugin(diagnostics::DiagnosticsPlugin);
        app.register_plugin(inspector::InspectorPlugin);
        app.register_plugin(viewport::ViewportPlugin);
        app.register_plugin(shell::ShellPlugin);
        app.register_plugin(window_chrome::WindowChromePlugin {
            decorated: self.decorated,
        });
        if let Some(path) = &self.project {
            app.get_resource_mut::<project::EditorCommands>()
                .expect("EditorCommands was just inserted")
                .0
                .push_back(project::EditorCommand::OpenProject(path.clone()));
        }
    }
}
