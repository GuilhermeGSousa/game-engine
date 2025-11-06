use std::collections::HashMap;

use essential::assets::Asset;
use glam::{Quat, Vec3};
use uuid::Uuid;

pub enum AnimationChanelOutput {
    Translation(Vec3),
    Rotation(Quat),
    Scale(Vec3),
}

impl AnimationChanelOutput {
    pub fn from_translation(translation: [f32; 3]) -> Self {
        Self::Translation(Vec3::from_array(translation))
    }

    pub fn from_rotation(rotation: [f32; 4]) -> Self {
        Self::Rotation(Quat::from_array(rotation))
    }

    pub fn from_scale(scale: [f32; 3]) -> Self {
        Self::Scale(Vec3::from_array(scale))
    }
}

pub struct AnimationChannel {
    time_samples: Vec<f32>,
    outputs: Vec<AnimationChanelOutput>,
}

impl Default for AnimationChannel {
    fn default() -> Self {
        Self {
            time_samples: Default::default(),
            outputs: Default::default(),
        }
    }
}

impl AnimationChannel {
    pub fn new(time_samples: Vec<f32>, outputs: Vec<AnimationChanelOutput>) -> Self {
        Self {
            time_samples,
            outputs,
        }
    }

    pub fn set_data(&mut self, time_samples: Vec<f32>, outputs: Vec<AnimationChanelOutput>) {
        self.time_samples = time_samples;
        self.outputs = outputs;
    }
}

pub struct AnimationClip {
    channels: HashMap<Uuid, AnimationChannel>,
}

impl Asset for AnimationClip {}

impl Default for AnimationClip {
    fn default() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }
}

impl AnimationClip {
    pub fn add_channel(&mut self, id: Uuid, channel: AnimationChannel) {
        self.channels.insert(id, channel);
    }
}
