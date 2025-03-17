use app::App;
use render::plugin::RenderPlugin;
use ui::plugin::UIPlugin;
use window::plugin::WindowPlugin;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run_game() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    App::empty()
        .register_plugin(WindowPlugin)
        .register_plugin(RenderPlugin)
        .register_plugin(UIPlugin)
        .run();
}
