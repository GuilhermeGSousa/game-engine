use super::{Asset, AssetId};

pub struct AssetHandle<A: Asset> {
    id: AssetId,
    marker: std::marker::PhantomData<A>,
}

impl<A: Asset> AssetHandle<A> {
    pub fn new(id: AssetId) -> Self {
        AssetHandle {
            id,
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
            marker: std::marker::PhantomData,
        }
    }
}

impl<A: Asset> Drop for AssetHandle<A> {
    fn drop(&mut self) {
        // Placeholder for any cleanup logic if needed
    }
}
