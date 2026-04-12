use app::{plugins::PluginsState, App};
use ecs::events::event_channel::EventChannel;
use input::Input;
use plugin::Window;
use winit::{application::ApplicationHandler, keyboard::PhysicalKey};

use winit::event::WindowEvent as WinitWindowEvent;

use crate::winit_events::WindowEvent;

pub mod input;
pub mod plugin;
pub mod winit_events;

pub fn run() {}

pub struct ApplicationWindowHandler {
    app: App,
    winit_events: Vec<winit::event::WindowEvent>,
}

impl ApplicationWindowHandler {
    pub fn new(app: App) -> Self {
        Self {
            app,
            winit_events: Vec::new(),
        }
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

        self.winit_events.push(event.clone());

        match event {
            WinitWindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WinitWindowEvent::RedrawRequested => {
                let event_channel = self
                    .app
                    .get_mut_resource::<EventChannel<WindowEvent>>()
                    .unwrap();

                self.winit_events
                    .drain(..)
                    .for_each(|e| event_channel.push_event(WindowEvent::new(e.clone())));

                if self.app.plugin_state() == PluginsState::Finished {
                    self.app.update();
                }

                let window = self.app.get_resource::<Window>().unwrap();
                window.request_redraw();
            }
            WinitWindowEvent::KeyboardInput { event, .. } => {
                let input = self.app.get_mut_resource::<Input>().unwrap();
                // Capture typed text (handles modifier keys, dead keys, etc.).
                // Only on press — not release — and only printable characters.
                if event.state == winit::event::ElementState::Pressed {
                    if let Some(text) = &event.text {
                        for c in text.chars() {
                            if !c.is_control() {
                                input.push_typed_char(c);
                            }
                        }
                    }
                }
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    input.update_key_input(PhysicalKey::Code(keycode), event.state);
                }
            }
            WinitWindowEvent::CursorMoved { position, .. } => {
                let input = self.app.get_mut_resource::<Input>().unwrap();
                input.update_mouse_position(position.x, position.y);
            }
            WinitWindowEvent::MouseInput { state, button, .. } => {
                let input = self.app.get_mut_resource::<Input>().unwrap();
                input.update_mouse_button(button, state);
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

        if self.app.plugin_state() == PluginsState::Ready {
            self.app.finish_plugin_build();
        }
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
        if let winit::event::DeviceEvent::MouseMotion { delta } = event {
            let input_state = self.app.get_mut_resource::<Input>().unwrap();
            input_state.update_mouse_delta(delta);
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
