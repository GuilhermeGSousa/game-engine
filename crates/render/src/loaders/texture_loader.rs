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
        _usage_settings: <Self::Asset as LoadableAsset>::UsageSettings,
    ) -> anyhow::Result<Self::Asset> {
        // TODO(follow-up): output root hard-coded as "res" (matches AssetPath's
        // "res/" rooting convention) rather than threaded through AssetLoadContext.
        let cooked_path = asset_cook::cooked_file_path_for_id(
            std::path::Path::new("res"),
            load_context.asset_id(),
        );
        let bytes = std::fs::read(&cooked_path).with_context(|| {
            format!(
                "failed to read cooked texture at '{}'",
                cooked_path.display()
            )
        })?;
        let cooked: crate::assets::cooked_texture::CookedTexture = bincode::deserialize(&bytes)?;
        Ok(Texture::from_cooked(cooked))
    }
}
