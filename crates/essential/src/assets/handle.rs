use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use crate::assets::AssetPath;

use super::{Asset, AssetId};
use crossbeam_channel::Sender;
use ecs::Event;

#[derive(Clone, Event)]
pub enum AssetLifetimeEvent {
    Dropped(AssetId, Option<AssetPath<'static>>),
}

impl AssetLifetimeEvent {
    pub fn id(&self) -> AssetId {
        match self {
            AssetLifetimeEvent::Dropped(id, _) => *id,
        }
    }
}

pub struct StrongAssetHandle {
    pub(crate) id: AssetId,
    pub(crate) path: Option<AssetPath<'static>>,
    pub(crate) lifetime_sender: Sender<AssetLifetimeEvent>,
}

impl Drop for StrongAssetHandle {
    fn drop(&mut self) {
        let path = self.path.clone();
        let _ = self
            .lifetime_sender
            .send(AssetLifetimeEvent::Dropped(self.id, path));
    }
}

impl Debug for StrongAssetHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StrongAssetHandle")
            .field("id", &self.id)
            .field("path", &self.path)
            .finish()
    }
}

pub struct AssetHandle<A: Asset> {
    handle: Arc<StrongAssetHandle>,
    _marker: PhantomData<A>,
}

impl<A: Asset> AssetHandle<A> {
    pub(crate) fn new(handle: Arc<StrongAssetHandle>) -> Self {
        Self {
            handle,
            _marker: PhantomData,
        }
    }

    pub fn id(&self) -> AssetId {
        self.handle.id
    }
}

impl<A: Asset> Clone for AssetHandle<A> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
            _marker: PhantomData,
        }
    }
}

impl<A: Asset> Debug for AssetHandle<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetHandle")
            .field("handle", &self.handle)
            .finish()
    }
}
