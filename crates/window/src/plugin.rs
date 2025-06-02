use ecs::resource::{ResMut, Resource};
use std::sync::Arc;

use crate::{input::Input, ApplicationWindowHandler};
use app::{plugins::Plugin, runner::AppExit, update_group::UpdateGroup, App};

#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;

use winit::{
    event_loop::{ControlFlow, EventLoop},
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window as WinitWindow,
};

#[derive(Resource)]
pub struct Window {
    pub window_handle: Arc<WinitWindow>,
    size: (u32, u32),
    should_resize: bool,
}

#[derive(Resource)]
pub struct WindowEventLoop(EventLoop<()>);

impl Window {
    pub fn new(window: WinitWindow) -> Self {
        Self {
            window_handle: Arc::new(window),
            size: (0, 0),
            should_resize: true,
        }
    }

    pub fn request_redraw(&self) {
        self.window_handle.request_redraw();
    }

    pub fn size(&self) -> (u32, u32) {
        let size = self.window_handle.inner_size();
        (size.width, size.height)
    }

    pub fn request_resize(&mut self, size: (u32, u32)) {
        self.size = size;
        self.should_resize = true;
    }

    pub fn should_resize(&self) -> bool {
        self.should_resize
    }

    pub fn clear_resize(&mut self) {
        self.should_resize = false;
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(
        &self,
    ) -> Result<winit::raw_window_handle::DisplayHandle<'_>, winit::raw_window_handle::HandleError>
    {
        self.window_handle.display_handle()
    }
}

impl HasWindowHandle for Window {
    fn window_handle(
        &self,
    ) -> Result<winit::raw_window_handle::WindowHandle<'_>, winit::raw_window_handle::HandleError>
    {
        self.window_handle.window_handle()
    }
}

pub struct WindowPlugin;

fn winit_runner(mut app: App) -> AppExit {
    let event_loop = app.remove_resource::<WindowEventLoop>().unwrap();

    let mut state = ApplicationWindowHandler::new(app);

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            event_loop.0.spawn_app(state);
        } else {
            let _ = event_loop.0.run_app(&mut state);
        }
    }

    AppExit::Success
}

fn update_input(mut input: ResMut<Input>) {
    input.update();
}

#[allow(deprecated)]
impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        let mut event_loop_builder = EventLoop::builder();

        let event_loop = event_loop_builder
            .build()
            .expect("Failed to build event loop");
        event_loop.set_control_flow(ControlFlow::Poll);
        let win_attr = WinitWindow::default_attributes().with_title("winit example");
        let window = event_loop
            .create_window(win_attr)
            .expect("create window err.");

        #[cfg(target_arch = "wasm32")]
        {
            // Winit prevents sizing with CSS, so we have to set
            // the size manually when on web.
            use winit::dpi::PhysicalSize;
            let _ = window.request_inner_size(PhysicalSize::new(450, 400));

            use winit::platform::web::WindowExtWebSys;
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let dst = doc.get_element_by_id("wasm-example")?;
                    let canvas = web_sys::Element::from(window.canvas()?);
                    dst.append_child(&canvas).ok()?;
                    Some(())
                })
                .expect("Couldn't append canvas to document body.");
        }

        app.insert_resource(Input::new());
        app.insert_resource(Window::new(window));
        app.insert_resource(WindowEventLoop(event_loop));
        app.add_system(UpdateGroup::Render, update_input);
        app.set_runner(winit_runner);
    }
}
