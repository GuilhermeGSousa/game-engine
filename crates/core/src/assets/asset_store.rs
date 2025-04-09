use std::collections::HashMap;

use ecs::resource::Resource;

use super::{handle::AssetHandle, Asset, AssetId};

#[derive(Resource)]
pub struct AssetStore<A: Asset + 'static> {
    assets: HashMap<AssetId, A>,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset + 'static> AssetStore<A> {
    pub fn get(&self, handle: &AssetHandle<A>) -> Option<&A> {
        self.assets.get(&handle.id())
    }
}
