use std::collections::HashMap;

use glam::Vec2;

pub enum AnimationBlackboardValue {
    Bool(bool),
    Int(u32),
    Float(f32),
    Vec2(Vec2),
}

#[derive(Default)]
pub struct AnimationBlackboard {
    values: HashMap<String, AnimationBlackboardValue>,
}

impl AnimationBlackboard {
    pub fn set(&mut self, key: impl Into<String>, value: AnimationBlackboardValue) {
        self.values.insert(key.into(), value);
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.values.get(key)? {
            AnimationBlackboardValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn get_int(&self, key: &str) -> Option<u32> {
        match self.values.get(key)? {
            AnimationBlackboardValue::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn get_float(&self, key: &str) -> Option<f32> {
        match self.values.get(key)? {
            AnimationBlackboardValue::Float(v) => Some(*v),
            _ => None,
        }
    }

    pub fn get_vec2(&self, key: &str) -> Option<Vec2> {
        match self.values.get(key)? {
            AnimationBlackboardValue::Vec2(v) => Some(*v),
            _ => None,
        }
    }
}
