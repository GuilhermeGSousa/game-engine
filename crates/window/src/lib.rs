use app::App;
use input::Input;
use plugin::Window;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    keyboard::PhysicalKey,
};

pub mod input;
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
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.app.update();
                let window = self.app.get_resource::<Window>().unwrap();
                window.request_redraw();
            }
            WindowEvent::Resized(size) => {
                let window = self.app.get_mut_resource::<Window>().unwrap();
                window.request_resize((size.width, size.height));
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(keycode),
                        ..
                    },
                ..
            } => {
                let input_state = self.app.get_mut_resource::<Input>().unwrap();
                input_state.update_key_input(PhysicalKey::Code(keycode), state);
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
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        match event {
            winit::event::DeviceEvent::MouseMotion { delta } => {
                let input_state = self.app.get_mut_resource::<Input>().unwrap();
                input_state.update_mouse_delta(delta);
            }
            _ => (),
        }
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
