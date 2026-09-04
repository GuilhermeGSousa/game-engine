use anyhow::Context;
use essential::assets::Asset;
use essential::assets::{asset_loader::AssetLoader, asset_server::AssetLoadContext, AssetPath};

use async_trait::async_trait;

use crate::assets::texture::Texture;

pub struct TextureLoader;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AssetLoader for TextureLoader {
    type Asset = Texture;

    async fn load(
        &self,
        path: AssetPath<'static>,
        load_context: &mut AssetLoadContext,
        _usage_settings: (),
    ) -> anyhow::Result<Self::Asset> {
        let bytes = essential::assets::utils::load_asset_bytes(
            load_context.cooked_root(),
            &path.address(),
            load_context.asset_id(),
            Texture::name(),
        )
        .await
        .with_context(|| "failed to read texture asset")?;
        let texture: Texture =
            bincode::deserialize(&bytes).with_context(|| "failed to deserialize texture asset")?;
        Ok(texture)
    }
}
