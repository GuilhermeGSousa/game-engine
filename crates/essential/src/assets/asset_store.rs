use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;

use ecs::resource::Resource;

use super::{
    handle::{AssetHandle, AssetLifetimeEvent},
    Asset, AssetId,
};

struct AssetStoreEntry<A: Asset> {
    pub(crate) asset: A,
    pub(crate) ref_count: i32,
}

#[derive(Resource)]
pub struct AssetStore<A: Asset + 'static> {
    assets: HashMap<AssetId, AssetStoreEntry<A>>,
    drop_sender: Sender<AssetLifetimeEvent>,
    drop_receiver: Receiver<AssetLifetimeEvent>,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset + 'static> AssetStore<A> {
    pub fn new() -> Self {
        let (lifetime_sender, lifetime_recv) = crossbeam_channel::unbounded();
        AssetStore {
            assets: HashMap::new(),
            drop_sender: lifetime_sender,
            drop_receiver: lifetime_recv,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn get(&self, handle: &AssetHandle<A>) -> Option<&A> {
        self.assets.get(&handle.id()).map(|entry| &entry.asset)
    }

    pub fn insert(&mut self, id: AssetId, asset: A)
    where
        A: 'static,
    {
        let entry = AssetStoreEntry {
            asset,
            ref_count: 1,
        };
        self.assets.insert(id, entry);
    }

    pub fn track_assets(&mut self) {
        for event in self.drop_receiver.try_iter() {
            if let Some(entry) = self.assets.get_mut(&event.id()) {
                match event {
                    AssetLifetimeEvent::Cloned(_) => entry.ref_count += 1,
                    AssetLifetimeEvent::Dropped(_) => entry.ref_count -= 1,
                }

                if entry.ref_count <= 0 {
                    self.assets.remove(&event.id());
                }
            }
        }
    }

    pub fn clone_drop_sender(&self) -> Sender<AssetLifetimeEvent> {
        self.drop_sender.clone()
    }
}
