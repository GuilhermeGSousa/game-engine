use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;

use ecs::resource::Resource;

use super::{
    handle::{AssetHandle, DropEvent},
    Asset, AssetId,
};

struct AssetStoreEntry<A: Asset> {
    pub(crate) asset: A,
    pub(crate) ref_count: i32,
}

#[derive(Resource)]
pub struct AssetStore<A: Asset + 'static> {
    assets: HashMap<AssetId, AssetStoreEntry<A>>,
    drop_sender: Sender<DropEvent>,
    drop_receiver: Receiver<DropEvent>,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset + 'static> AssetStore<A> {
    pub fn new() -> Self {
        let (drop_sender, drop_receiver) = crossbeam_channel::unbounded();
        AssetStore {
            assets: HashMap::new(),
            drop_sender,
            drop_receiver,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn get(&self, handle: &AssetHandle<A>) -> Option<&A> {
        self.assets.get(&handle.id()).map(|entry| &entry.asset)
    }

    pub fn track_assets(&mut self) {
        for event in self.drop_receiver.try_iter() {
            if let Some(entry) = self.assets.get_mut(&event.id) {
                entry.ref_count -= 1;
                if entry.ref_count <= 0 {
                    self.assets.remove(&event.id);
                }
            }
        }
    }
}
