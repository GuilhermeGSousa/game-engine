use cfg_if::cfg_if;
use ecs::resource::Resource;
use glam::Vec2;
use std::collections::HashMap;
use winit::{event::ElementState, keyboard::PhysicalKey};

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
    #[cfg(target_arch = "wasm32")]
    previous_mouse_delta: Vec2,
}

impl Input {
    pub fn new() -> Self {
        Self {
            input_map: HashMap::new(),
            mouse_delta: Vec2::ZERO,
            previous_mouse_delta: Vec2::ZERO,
        }
    }

    pub fn get_key_state(&self, key: PhysicalKey) -> InputState {
        match self.input_map.get(&key) {
            Some(state) => state.clone(),
            None => InputState::Up,
        }
    }

    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }

    pub fn update(&mut self) {
        for (_, state) in self.input_map.iter_mut() {
            match state {
                InputState::Pressed => *state = InputState::Down,
                InputState::Released => *state = InputState::Up,
                _ => {}
            }
        }
        self.mouse_delta = Vec2::ZERO; // Reset mouse delta after processing
    }

    pub fn update_key_input(&mut self, key: PhysicalKey, state: ElementState) {
        // If the key is not in the map, add it with the current state
        if !self.input_map.contains_key(&key) {
            match state {
                ElementState::Pressed => self.input_map.insert(key, InputState::Pressed),
                ElementState::Released => self.input_map.insert(key, InputState::Released),
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
