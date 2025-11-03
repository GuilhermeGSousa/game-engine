use ecs::entity::Entity;
use essential::assets::Asset;

pub enum AnimationChanelOutput {}

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

pub struct AnimationClip {
    channels: Vec<AnimationChannel>,
}

impl Asset for AnimationClip {}

impl Default for AnimationClip {
    fn default() -> Self {
        Self {
            channels: Vec::new(),
        }
    }
}

impl AnimationClip {
    pub fn add_channel(&mut self, channel: AnimationChannel) {
        self.channels.push(channel);
    }
}
