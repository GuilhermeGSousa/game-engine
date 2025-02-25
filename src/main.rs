use app::App;
use render::plugin::RenderPlugin;
use window::plugin::WindowPlugin;

fn main() {
    App::empty()
        .register_plugin(WindowPlugin)
        .register_plugin(RenderPlugin)
        .run();
}
