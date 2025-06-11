use std::ops::{Deref, DerefMut};

use ecs::events::Event;

#[derive(Event)]
pub struct WinitEvent(winit::event::WindowEvent);

impl WinitEvent {
    pub fn new(event: winit::event::WindowEvent) -> Self {
        Self(event)
    }
}

impl Deref for WinitEvent {
    type Target = winit::event::WindowEvent;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WinitEvent {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
