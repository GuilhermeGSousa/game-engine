pub mod render_material;
pub mod render_mesh;
pub mod render_texture;
pub mod render_window;

use std::collections::HashMap;

use app::plugins::Plugin;
use ecs::{
    resource::{ResMut, Resource},
    system::system_input::{StaticSystemInput, SystemInput, SystemInputData},
};
use essential::assets::{asset_store::AssetStore, Asset, AssetId};

pub enum AssetPreparationError {
    NotReady,
}

pub(crate) trait RenderAsset: Sized + 'static {
    type SourceAsset: Asset;
    type PreparationParams: SystemInput;

    fn prepare_asset(
        source_asset: &Self::SourceAsset,
        params: &mut SystemInputData<Self::PreparationParams>,
    ) -> Result<Self, AssetPreparationError>;
}

pub(crate) fn prepare_render_asset<A: RenderAsset>(
    mut params: StaticSystemInput<<A as RenderAsset>::PreparationParams>,
    asset_store: ResMut<AssetStore<A::SourceAsset>>,
    mut render_assets: ResMut<RenderAssets<A>>,
) {
    for (asset_id, asset) in asset_store.into_iter() {
        let prepared_asset = A::prepare_asset(asset, &mut params);

        match prepared_asset {
            Ok(prepared_asset) => render_assets.insert(*asset_id, prepared_asset),
            Err(_) => {}
        };
    }
}

#[derive(Resource)]
pub(crate) struct RenderAssets<A: RenderAsset + 'static>(HashMap<AssetId, A>);

impl<A: RenderAsset + 'static> RenderAssets<A> {
    pub fn new() -> Self {
        RenderAssets(HashMap::new())
    }

    pub fn insert(&mut self, id: AssetId, asset: A) {
        self.0.insert(id, asset);
    }

    pub fn get(&self, id: &AssetId) -> Option<&A> {
        self.0.get(id)
    }
}

pub(crate) struct RenderAssetPlugin<A: RenderAsset> {
    _marker: std::marker::PhantomData<A>,
}

impl<A: RenderAsset> RenderAssetPlugin<A> {
    pub fn new() -> Self {
        RenderAssetPlugin {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<A: RenderAsset + 'static> Plugin for RenderAssetPlugin<A> {
    fn build(&self, app: &mut app::App) {
        app.insert_resource(RenderAssets::<A>::new());
        app.add_system(
            app::update_group::UpdateGroup::Render,
            prepare_render_asset::<A>,
        );
    }
}
