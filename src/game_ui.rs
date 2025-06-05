use ecs::resource::ResMut;
use ui::resources::UIRenderer;

pub(crate) fn render_ui(ui_renderer: ResMut<UIRenderer>) {
    // Render UI
    egui::Window::new("winit + egui + wgpu says hello!")
        .resizable(true)
        .vscroll(true)
        .default_open(false)
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
