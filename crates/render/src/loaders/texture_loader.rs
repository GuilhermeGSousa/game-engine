use anyhow::Context;
use essential::assets::{
    asset_loader::AssetLoader, asset_server::AssetLoadContext, AssetPath, LoadableAsset,
};

use async_trait::async_trait;

use crate::assets::texture::Texture;

pub struct TextureLoader;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AssetLoader for TextureLoader {
    type Asset = Texture;

    async fn load(
        &self,
        _path: AssetPath<'static>,
        load_context: &mut AssetLoadContext,
        // NOTE: usage_settings is deliberately ignored — the cooked
        // CookedTexture.srgb flag now determines the format.
        // TextureUsageSettings::linear() from a caller has no effect on a
        // cooked texture.
        _usage_settings: <Self::Asset as LoadableAsset>::UsageSettings,
    ) -> anyhow::Result<Self::Asset> {
        let bytes = essential::assets::utils::load_cooked_asset_bytes(
            load_context.cooked_root(),
            load_context.asset_id(),
        )
        .await
        .with_context(|| "failed to read cooked texture")?;
        let cooked: crate::assets::cooked_texture::CookedTexture =
            bincode::deserialize(&bytes).with_context(|| "failed to deserialize cooked texture")?;
        Ok(Texture::from_cooked(cooked))
    }
}
