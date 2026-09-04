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

/// Reads an asset's payload, preferring a content asset at `<root>/<address>`
/// and falling back to the cooked `<root>/.cooked/<hex>.bin` layout.
///
/// The fallback exists only while both pipelines coexist; it is removed when
/// the manifest cook is deleted.
pub async fn load_asset_bytes(
    root: &CookedAssetRoot,
    address: &str,
    id: AssetId,
    expected_kind: &str,
) -> anyhow::Result<Vec<u8>> {
    // A path-less load (load_by_id) arrives as an empty address; a cook-style
    // "<source>#<sub>" address is never a content asset and its fragment
    // corrupts the path join / request URL. Both go straight to the cooked layout.
    if address.is_empty() || address.contains('#') {
        return load_cooked_asset_bytes(root, id).await;
    }

    let Some(raw) = try_read_relative(root, address).await? else {
        return load_cooked_asset_bytes(root, id).await;
    };

    let (header, payload) = crate::assets::content::read_content_asset(&raw)
        .with_context(|| format!("malformed content asset '{address}'"))?;
    if header.kind != expected_kind {
        anyhow::bail!(
            "content asset '{address}' holds a {} but a {expected_kind} was requested",
            header.kind
        );
    }
    Ok(payload.to_vec())
}

/// Reads `<root>/<relative>`, returning `Ok(None)` when it simply is not
/// there (a missing file natively, a non-success status on wasm) so the
/// caller can fall back. Any other failure is a real error.
async fn try_read_relative(
    root: &CookedAssetRoot,
    relative: &str,
) -> anyhow::Result<Option<Vec<u8>>> {
    match root {
        CookedAssetRoot::Directory(dir) => {
            let path = dir.join(relative);
            cfg_if! {
                if #[cfg(target_arch = "wasm32")] {
                    let _ = path;
                    Ok(None)
                } else {
                    // A path-less or otherwise non-file join (the root dir
                    // itself, a file where a directory was expected) is
                    // absence, not failure, so the caller can fall back.
                    match async_fs::read(&path).await {
                        Ok(bytes) => Ok(Some(bytes)),
                        Err(err)
                            if matches!(
                                err.kind(),
                                std::io::ErrorKind::NotFound
                                    | std::io::ErrorKind::IsADirectory
                                    | std::io::ErrorKind::NotADirectory
                                    | std::io::ErrorKind::InvalidInput
                            ) =>
                        {
                            Ok(None)
                        }
                        Err(err) => Err(anyhow::Error::new(err))
                            .with_context(|| format!("failed to read '{}'", path.display())),
                    }
                }
            }
        }
        CookedAssetRoot::UrlBase(base) => {
            cfg_if! {
                if #[cfg(target_arch = "wasm32")] {
                    let url = format!("{base}/{relative}");
                    let response = reqwest::get(&url)
                        .await
                        .with_context(|| format!("HTTP request for '{url}' failed"))?;
                    if !response.status().is_success() {
                        return Ok(None);
                    }
                    let bytes = response
                        .bytes()
                        .await
                        .with_context(|| format!("failed to read response body for '{url}'"))?;
                    Ok(Some(bytes.to_vec()))
                } else {
                    let _ = relative;
                    anyhow::bail!(
                        "CookedAssetRoot::UrlBase is only supported on wasm32 (base '{base}')"
                    )
                }
            }
        }
    }
}
