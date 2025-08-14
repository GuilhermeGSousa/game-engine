use std::{marker::PhantomData, sync::Arc};

use super::{Asset, AssetId};
use crossbeam_channel::Sender;

pub enum AssetLifetimeEvent {
    Dropped(AssetId),
}

impl AssetLifetimeEvent {
    pub fn id(&self) -> AssetId {
        match self {
            AssetLifetimeEvent::Dropped(id) => *id,
        }
    }
}

pub struct StrongAssetHandle {
    pub(crate) id: AssetId,
    pub(crate) lifetime_sender: Sender<AssetLifetimeEvent>,
}

impl Drop for StrongAssetHandle {
    fn drop(&mut self) {
        let _ = self
            .lifetime_sender
            .send(AssetLifetimeEvent::Dropped(self.id));
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
