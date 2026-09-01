use std::path::Path;

use asset_cook::{ImportContext, ImportError, Importer};
use image::GenericImageView;

use crate::assets::cooked_texture::CookedTexture;

pub struct ImageImporter;

impl Importer for ImageImporter {
    fn supported_extensions(&self) -> &'static [&'static str] {
        &["png", "jpg", "jpeg"]
    }

    fn import(&self, source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError> {
        let img = image::open(source_path).map_err(|err| ImportError::MalformedSource {
            source_path: source_path.to_path_buf(),
            message: err.to_string(),
        })?;

        let (width, height) = img.dimensions();
        let cooked = CookedTexture {
            width,
            height,
            // TODO(asset-import-pipeline): colour space hard-coded sRGB —
            // standalone linear textures (normal/metallic-roughness .png)
            // can't be cooked correctly yet; needs a per-entry hint in
            // assets.toml or a filename convention.
            srgb: true,
            pixels: img.to_rgba8().into_raw(),
        };

        ctx.emit("main", &cooked)
            .map_err(|err| ImportError::MalformedSource {
                source_path: source_path.to_path_buf(),
                message: format!("{err:?}"),
            })?;

        Ok(())
    }
}
