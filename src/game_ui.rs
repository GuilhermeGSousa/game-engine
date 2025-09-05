use ecs::resource::Res;
use ui::resources::UIRenderer;

pub(crate) fn render_ui(ui_renderer: Res<UIRenderer>) {
    // Render UI
    egui::Window::new("winit + egui + wgpu says hello!")
        .resizable(false)
        .vscroll(true)
        .default_open(false)
        .fade_in(true)
        .movable(false)
        .show(ui_renderer.context(), |ui| {
            ui.label("Label!");

            if ui.button("Button!").clicked() {
                println!("boom!")
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Pixels per point: {}",
                    ui_renderer.context().pixels_per_point()
                ));
            });
        });
}
