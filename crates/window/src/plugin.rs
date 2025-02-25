use std::sync::Arc;

use app::{plugin::Plugin, runner::AppExit, App};
use bevy_ecs::prelude::*;
use winit::{
    event_loop::{ControlFlow, EventLoop},
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window as WinitWindow,
};

use crate::ApplicationWindowHandler;

#[derive(Resource)]
pub struct Window {
    pub window_handle: Arc<WinitWindow>,
    size: (u32, u32),
    should_resize: bool,
}

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
    let event_loop = app.remove_non_send_resource::<EventLoop<()>>().unwrap();

    let mut state = ApplicationWindowHandler::new(app);

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            event_loop.spawn_app(state);
            AppExit::Success
        } else {
            let _ = event_loop.run_app(&mut state);
        }
    }
    AppExit::Success
}

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

        app.insert_resource(Window::new(window));
        app.insert_non_send_resource(event_loop);
        app.set_runner(winit_runner);
    }
}
