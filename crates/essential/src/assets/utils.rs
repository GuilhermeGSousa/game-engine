use anyhow::Context;
use cfg_if::cfg_if;

use super::{AssetId, CookedAssetRoot};

/// Reads the cooked bytes for `id` from `root`. This is the runtime mirror of
/// `asset_cook::cooked_file_path_for_id` — both resolve an AssetId to
/// `.cooked/<simple-hex>.bin`, and they must not drift apart.
pub async fn load_cooked_asset_bytes(
    root: &CookedAssetRoot,
    id: AssetId,
) -> anyhow::Result<Vec<u8>> {
    let file_name = format!("{}.bin", id.simple_hex());

    match root {
        CookedAssetRoot::Directory(dir) => {
            let path = dir.join(".cooked").join(&file_name);
            cfg_if! {
                if #[cfg(target_arch = "wasm32")] {
                    anyhow::bail!(
                        "CookedAssetRoot::Directory is not supported on wasm32 ('{}')",
                        path.display()
                    )
                } else {
                    async_fs::read(&path)
                        .await
                        .with_context(|| {
                            format!("failed to read cooked asset '{}'", path.display())
                        })
                }
            }
        }
        CookedAssetRoot::UrlBase(base) => {
            cfg_if! {
                if #[cfg(target_arch = "wasm32")] {
                    let url = format!("{base}/.cooked/{file_name}");
                    let bytes = reqwest::get(&url)
                        .await
                        .with_context(|| format!("HTTP request for cooked asset '{url}' failed"))?
                        .bytes()
                        .await
                        .with_context(|| format!("failed to read response body for '{url}'"))?
                        .to_vec();
                    Ok(bytes)
                } else {
                    anyhow::bail!(
                        "CookedAssetRoot::UrlBase is only supported on wasm32 (base '{base}')"
                    )
                }
            }
        }
    }
}
