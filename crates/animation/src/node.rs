use std::ops::Deref;

use essential::assets::handle::AssetHandle;

use crate::clip::AnimationClip;

pub trait AnimationGraphNode: Sync + Send {}

pub struct RootAnimationNode;

impl AnimationGraphNode for RootAnimationNode {}

pub struct AnimationClipNode(AssetHandle<AnimationClip>);

impl AnimationGraphNode for AnimationClipNode {}

impl Deref for AnimationClipNode {
    type Target = AssetHandle<AnimationClip>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
