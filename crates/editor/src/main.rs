use app::{
    App,
    plugins::{AssetManagerPlugin, TimePlugin},
};
use ecs::resource::{ResMut, Resource};
use egui_dock::{DockArea, DockState, NodeIndex, Style};
use render::plugin::RenderPlugin;
use ui::{plugin::UIPlugin, resources::UIRenderer};
use window::plugin::WindowPlugin;

fn main() {
    let mut app = App::empty();
    app.register_plugin(TimePlugin)
        .register_plugin(AssetManagerPlugin)
        .register_plugin(WindowPlugin)
        .register_plugin(RenderPlugin)
        .register_plugin(UIPlugin)
        .insert_resource(EditorDockState::default())
        .add_system(app::update_group::UpdateGroup::Render, render_ui)
        .run();
}

#[derive(Resource)]
pub(crate) struct EditorDockState {
    pub(crate) tree: DockState<String>,
}

impl Default for EditorDockState {
    fn default() -> Self {
        let mut tree = DockState::new(vec!["tab1".to_owned(), "tab2".to_owned()]);

        // You can modify the tree before constructing the dock
        let [a, b] =
            tree.main_surface_mut()
                .split_left(NodeIndex::root(), 0.3, vec!["tab3".to_owned()]);
        let [_, _] = tree
            .main_surface_mut()
            .split_below(a, 0.7, vec!["tab4".to_owned()]);
        let [_, _] = tree
            .main_surface_mut()
            .split_below(b, 0.5, vec!["tab5".to_owned()]);

        Self { tree }
    }
}

struct TabViewer {}

impl egui_dock::TabViewer for TabViewer {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        (&*tab).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.label(format!("Content of {tab}"));
    }
}

pub(crate) fn render_ui(ui_renderer: ResMut<UIRenderer>, mut state: ResMut<EditorDockState>) {
    // Render UI

    DockArea::new(&mut state.tree)
        .style(Style::from_egui(ui_renderer.context().style().as_ref()))
        .show(ui_renderer.context(), &mut TabViewer {});
}
