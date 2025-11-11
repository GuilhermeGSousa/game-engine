use std::collections::{HashMap, hash_map::Keys};

use essential::{assets::Asset, transform::Transform};
use glam::{Quat, Vec3};
use uuid::Uuid;

pub enum AnimationChanelOutput {
    Translation(Vec<Vec3>),
    Rotation(Vec<Quat>),
    Scale(Vec<Vec3>),
}

impl AnimationChanelOutput {
    pub fn from_translation(translations: impl Iterator<Item = [f32; 3]>) -> Self {
        Self::Translation(
            translations
                .map(|val| Vec3::from_array(val))
                .collect::<Vec<_>>(),
        )
    }

    pub fn from_rotation(rotations: impl Iterator<Item = [f32; 4]>) -> Self {
        Self::Rotation(
            rotations
                .map(|val| Quat::from_array(val))
                .collect::<Vec<_>>(),
        )
    }

    pub fn from_scale(scales: impl Iterator<Item = [f32; 3]>) -> Self {
        Self::Scale(scales.map(|val| Vec3::from_array(val)).collect::<Vec<_>>())
    }
}

pub struct AnimationChannel {
    time_samples: Vec<f32>,
    outputs: AnimationChanelOutput,
}

impl AnimationChannel {
    pub fn new(time_samples: Vec<f32>, outputs: AnimationChanelOutput) -> Self {
        Self {
            time_samples,
            outputs,
        }
    }

    pub fn interpolate(&self, current_time: f32, transform: &mut Transform) {
        if self.time_samples.is_empty() {
            return;
        }

        let Ok(from_index) = self
            .time_samples
            .binary_search_by(|val| val.total_cmp(&current_time))
        else {
            return;
        };

        let Some(after_time) = self.time_samples.get(from_index + 1) else {
            return;
        };

        let before_time = self.time_samples[from_index];

        let normalized_time = current_time / (after_time - before_time);
        match &self.outputs {
            AnimationChanelOutput::Translation(pos) => {
                let before_pos = pos[from_index];
                let after_pos = pos[from_index + 1];

                transform.translation = before_pos.lerp(after_pos, normalized_time);
            }
            AnimationChanelOutput::Rotation(rot) => {
                let before_rot = rot[from_index];
                let after_rot = rot[from_index + 1];

                transform.rotation = before_rot.slerp(after_rot, normalized_time);
            }
            AnimationChanelOutput::Scale(scl) => {
                let before_scl = scl[from_index];
                let after_scl = scl[from_index + 1];

                transform.scale = before_scl.lerp(after_scl, normalized_time);
            }
        }
    }

    pub fn duration(&self) -> Option<f32> {
        self.time_samples.last().copied()
    }
}

#[derive(Asset)]
pub struct AnimationClip {
    channels: HashMap<Uuid, Vec<AnimationChannel>>,
    duration: f32,
}

impl Default for AnimationClip {
    fn default() -> Self {
        Self {
            channels: HashMap::new(),
            duration: 0.0,
        }
    }
}

impl AnimationClip {
    pub fn add_channel(&mut self, id: Uuid, channel: AnimationChannel) {
        if let Some(channel_duration) = channel.duration() {
            self.duration = self.duration.max(channel_duration);
        }

        self.channels.entry(id).or_default().push(channel);
    }

    pub fn target_ids(&self) -> Keys<'_, Uuid, Vec<AnimationChannel>> {
        self.channels.keys()
    }

    pub fn get_channels(&self, id: &Uuid) -> Option<&Vec<AnimationChannel>> {
        self.channels.get(id)
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }
}
