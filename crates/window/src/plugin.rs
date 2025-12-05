use ecs::resource::{ResMut, Resource};
use std::sync::Arc;

use crate::{input::Input, winit_events::WinitEvent, ApplicationWindowHandler};
use app::{
    plugins::{Plugin, PluginsState},
    runner::AppExit,
    update_group::UpdateGroup,
    App,
};

#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;

use winit::{
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window as WinitWindow,
};

#[derive(Resource)]
pub struct Window {
    pub window_handle: Arc<WinitWindow>,
    size: (u32, u32),
}

#[derive(Resource)]
pub struct WindowEventLoopProxy(EventLoopProxy<()>);

impl Window {
    pub fn new(window: WinitWindow) -> Self {
        Self {
            window_handle: Arc::new(window),
            size: (0, 0),
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

fn winit_runner(mut app: App, event_loop: EventLoop<()>) -> AppExit {
    if app.plugin_state() == PluginsState::Ready {
        app.finish_plugin_build();
    }

    let state = ApplicationWindowHandler::new(app);

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            event_loop.0.spawn_app(state);
        } else {
            let mut state = state;
            let _ = event_loop.run_app(&mut state);
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
        app.register_event::<WinitEvent>();

        let mut event_loop_builder = EventLoop::builder();

        let event_loop = event_loop_builder
            .build()
            .expect("Failed to build event loop");
        event_loop.set_control_flow(ControlFlow::Poll);

        let win_attr = WinitWindow::default_attributes().with_title("winit example");

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowAttributesExtWebSys;
            win_attr = &mut win_attr;
            win_attr = win_attr.with_append(true);
        }

        let window = event_loop
            .create_window(win_attr)
            .expect("create window err.");
        
        window.set_cursor_visible(true);
        app.insert_resource(Input::new());
        app.insert_resource(Window::new(window));
        app.insert_resource(WindowEventLoopProxy(event_loop.create_proxy()));

        app.add_system(UpdateGroup::Render, update_input);
        app.set_runner(|app| winit_runner(app, event_loop));
    }
}
