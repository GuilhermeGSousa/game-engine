use ecs::{Component, Entity};
use essential::assets::{handle::AssetHandle, Asset};
use glam::Mat4;
use uuid::Uuid;

#[derive(Asset)]
pub struct Skeleton {
    pub inverse_bindposes: Box<[Mat4]>,
}

impl From<Vec<Mat4>> for Skeleton {
    fn from(value: Vec<Mat4>) -> Self {
        Self {
            inverse_bindposes: value.into_boxed_slice(),
        }
    }
}

#[derive(Component)]
pub struct SkeletonComponent {
    skeleton: AssetHandle<Skeleton>,
    bones: Vec<Entity>,
    bone_ids: Vec<Uuid>,
}

impl SkeletonComponent {
    pub fn new(skeleton: AssetHandle<Skeleton>, bones: Vec<Entity>, bone_ids: Vec<Uuid>) -> Self {
        Self {
            skeleton,
            bones,
            bone_ids,
        }
    }

    pub fn skeleton(&self) -> &AssetHandle<Skeleton> {
        &self.skeleton
    }

    pub fn bones(&self) -> &[Entity] {
        &self.bones
    }

    pub fn bone_ids(&self) -> &[Uuid] {
        &self.bone_ids
    }
}
