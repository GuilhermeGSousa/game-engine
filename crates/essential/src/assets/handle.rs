use super::{Asset, AssetId};
use crossbeam_channel::Sender;

pub enum AssetLifetimeEvent {
    Cloned(AssetId),
    Dropped(AssetId),
}

impl AssetLifetimeEvent {
    pub fn id(&self) -> AssetId {
        match self {
            AssetLifetimeEvent::Cloned(id) => *id,
            AssetLifetimeEvent::Dropped(id) => *id,
        }
    }
}

pub struct AssetHandle<A: Asset> {
    id: AssetId,
    lifetime_sender: Sender<AssetLifetimeEvent>,
    marker: std::marker::PhantomData<A>,
}

impl<A: Asset> AssetHandle<A> {
    pub fn new(id: AssetId, lifetime_sender: Sender<AssetLifetimeEvent>) -> Self {
        AssetHandle {
            id,
            lifetime_sender,
            marker: std::marker::PhantomData,
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }
}

impl<A: Asset> Clone for AssetHandle<A> {
    fn clone(&self) -> Self {
        let _ = self
            .lifetime_sender
            .send(AssetLifetimeEvent::Cloned(self.id));
        AssetHandle {
            id: self.id,
            lifetime_sender: self.lifetime_sender.clone(),
            marker: std::marker::PhantomData,
        }
    }
}

impl<A: Asset> Drop for AssetHandle<A> {
    fn drop(&mut self) {
        let _ = self
            .lifetime_sender
            .send(AssetLifetimeEvent::Dropped(self.id));
    }
}
