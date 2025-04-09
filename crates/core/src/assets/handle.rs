use super::{Asset, AssetId};
use crossbeam_channel::Sender;

pub struct DropEvent {
    pub id: AssetId,
}

pub struct AssetHandle<A: Asset> {
    id: AssetId,
    drop_sender: Sender<DropEvent>,
    marker: std::marker::PhantomData<A>,
}

impl<A: Asset> AssetHandle<A> {
    pub fn new(id: AssetId, drop_sender: Sender<DropEvent>) -> Self {
        AssetHandle {
            id,
            drop_sender,
            marker: std::marker::PhantomData,
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }
}

impl<A: Asset> Clone for AssetHandle<A> {
    fn clone(&self) -> Self {
        AssetHandle {
            id: self.id,
            drop_sender: self.drop_sender.clone(),
            marker: std::marker::PhantomData,
        }
    }
}

impl<A: Asset> Drop for AssetHandle<A> {
    fn drop(&mut self) {
        let _ = self.drop_sender.send(DropEvent { id: self.id });
    }
}
