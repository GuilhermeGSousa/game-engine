use app::App;
use plugin::Window;
use winit::{application::ApplicationHandler, event::WindowEvent};

pub mod plugin;

pub fn run() {}

pub struct ApplicationWindowHandler {
    app: App,
}

impl ApplicationWindowHandler {
    pub fn new(app: App) -> Self {
        Self { app: app }
    }
}

impl ApplicationHandler for ApplicationWindowHandler {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _ = event_loop;
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let _ = window_id;
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                println!("The window requested a redraw");
                self.app.update();
                let window = self.app.get_resource::<Window>().unwrap();
                window.request_redraw();
            }
            WindowEvent::Resized(size) => {
                println!("The window was resized to {:?}", size);
                let window = self.app.get_mut_resource::<Window>().unwrap();
                window.request_resize((size.width, size.height));
            }
            _ => (),
        }
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        let _ = (event_loop, cause);
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: ()) {
        let _ = (event_loop, event);
    }

    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let _ = (event_loop, device_id, event);
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _ = event_loop;
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_window() {
        super::run();
    }
}
