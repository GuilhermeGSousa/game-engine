use std::{marker::PhantomData, sync::Arc};

use crate::assets::AssetPath;

use super::{Asset, AssetId};
use crossbeam_channel::Sender;

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

pub struct AssetHandle<A: Asset> {
    handle: Arc<StrongAssetHandle>,
    _marker: PhantomData<A>,
}

impl<A: Asset> AssetHandle<A> {
    pub(crate) fn new(handle: Arc<StrongAssetHandle>) -> Self {
        Self {
            handle: handle,
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
