use cfg_if::cfg_if;
use ecs::resource::Resource;
use glam::Vec2;
use std::collections::HashMap;
use winit::{event::ElementState, keyboard::PhysicalKey};

// Re-export so that crates that depend on `window` can reference
// `MouseButton` without needing a direct `winit` dependency.
pub use winit::event::MouseButton;

#[derive(Clone, Copy, PartialEq)]
pub enum InputState {
    Pressed,
    Down,
    Released,
    Up,
}

#[derive(Resource)]
pub struct Input {
    input_map: HashMap<winit::keyboard::PhysicalKey, InputState>,
    mouse_delta: Vec2,
    mouse_position: Vec2,
    mouse_buttons: HashMap<MouseButton, InputState>,
    #[cfg(target_arch = "wasm32")]
    previous_mouse_delta: Vec2,
}

/// Advances a single [`InputState`] by one tick:
/// `Pressed → Down`, `Released → Up`, everything else is unchanged.
fn advance_input_state(state: &mut InputState) {
    match state {
        InputState::Pressed => *state = InputState::Down,
        InputState::Released => *state = InputState::Up,
        _ => {}
    }
}

impl Input {
    pub fn new() -> Self {
        Self {
            input_map: HashMap::new(),
            mouse_delta: Vec2::ZERO,
            mouse_position: Vec2::ZERO,
            mouse_buttons: HashMap::new(),
            #[cfg(target_arch = "wasm32")]
            previous_mouse_delta: Vec2::ZERO,
        }
    }

    pub fn get_key_state(&self, key: PhysicalKey) -> InputState {
        match self.input_map.get(&key) {
            Some(state) => *state,
            None => InputState::Up,
        }
    }

    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }

    /// Returns the current cursor position in physical pixels, with
    /// `(0, 0)` at the top-left of the window.
    pub fn mouse_position(&self) -> Vec2 {
        self.mouse_position
    }

    /// Returns the [`InputState`] for the given mouse button.
    pub fn get_mouse_button_state(&self, button: MouseButton) -> InputState {
        match self.mouse_buttons.get(&button) {
            Some(state) => *state,
            None => InputState::Up,
        }
    }

    pub fn update(&mut self) {
        for (_, state) in self.input_map.iter_mut() {
            advance_input_state(state);
        }
        for (_, state) in self.mouse_buttons.iter_mut() {
            advance_input_state(state);
        }
        self.mouse_delta = Vec2::ZERO; // Reset mouse delta after processing
    }

    pub fn update_key_input(&mut self, key: PhysicalKey, state: ElementState) {
        if let std::collections::hash_map::Entry::Vacant(e) = self.input_map.entry(key) {
            match state {
                ElementState::Pressed => {
                    e.insert(InputState::Pressed);
                }
                ElementState::Released => {
                    e.insert(InputState::Released);
                }
            };
            return;
        }

        match state {
            ElementState::Pressed => {
                if self.input_map.get(&key) == Some(&InputState::Up) {
                    self.input_map.insert(key, InputState::Pressed);
                }
            }
            ElementState::Released => {
                if self.input_map.get(&key) == Some(&InputState::Down) {
                    self.input_map.insert(key, InputState::Released);
                }
            }
        }
    }

    pub fn update_mouse_position(&mut self, position: (f64, f64)) {
        self.mouse_position = Vec2::new(position.0 as f32, position.1 as f32);
    }

    pub fn update_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        if let std::collections::hash_map::Entry::Vacant(e) = self.mouse_buttons.entry(button) {
            match state {
                ElementState::Pressed => {
                    e.insert(InputState::Pressed);
                }
                ElementState::Released => {
                    e.insert(InputState::Released);
                }
            };
            return;
        }

        match state {
            ElementState::Pressed => {
                if self.mouse_buttons.get(&button) == Some(&InputState::Up) {
                    self.mouse_buttons.insert(button, InputState::Pressed);
                }
            }
            ElementState::Released => {
                if self.mouse_buttons.get(&button) == Some(&InputState::Down) {
                    self.mouse_buttons.insert(button, InputState::Released);
                }
            }
        }
    }

    pub fn update_mouse_delta(&mut self, delta: (f64, f64)) {
        let delta = Vec2::new(delta.0 as f32, delta.1 as f32);

        cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                self.mouse_delta = delta - self.previous_mouse_delta;
                self.previous_mouse_delta = delta;
            }
            else
            {
                self.mouse_delta = delta;
            }
        }
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}
