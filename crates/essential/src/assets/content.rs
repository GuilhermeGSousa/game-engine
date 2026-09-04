//! Framing for game-ready content assets: `magic | u32 header_len |
//! bincode(ContentAssetHeader) | payload (verbatim)`. The header is read
//! without touching the payload, so a future asset registry can index a
//! whole content tree by scanning headers alone.
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

use super::{Asset, AssetId};

/// Leading bytes of every content asset file.
pub const CONTENT_ASSET_MAGIC: [u8; 4] = *b"GRDY";

pub const CONTENT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentAssetHeader {
    pub format_version: u32,
    /// Identity. Today always `AssetId::from_path(<project-relative path>)`,
    /// which is recomputable from the path — it is stored so a future editor
    /// can mint a stable id here instead without a format break.
    pub asset_id: AssetId,
    /// Outbound references, so a registry scan never reads payloads.
    pub references: Vec<AssetId>,
    /// Authoritative type tag; must equal the loading type's `Asset::name()`.
    pub kind: String,
}

pub fn write_content_asset(header: &ContentAssetHeader, payload: &[u8]) -> anyhow::Result<Vec<u8>> {
    let header_bytes =
        bincode::serialize(header).context("failed to serialize content asset header")?;

    let header_len =
        u32::try_from(header_bytes.len()).context("content asset header exceeds 4 GiB")?;

    let mut out = Vec::with_capacity(8 + header_bytes.len() + payload.len());
    out.extend_from_slice(&CONTENT_ASSET_MAGIC);
    out.extend_from_slice(&header_len.to_le_bytes());
    out.extend_from_slice(&header_bytes);
    out.extend_from_slice(payload);
    Ok(out)
}

pub fn read_content_asset(bytes: &[u8]) -> anyhow::Result<(ContentAssetHeader, &[u8])> {
    if bytes.len() < 8 || bytes[..4] != CONTENT_ASSET_MAGIC {
        bail!("not a content asset: missing the GRDY magic prefix");
    }

    let header_len = u32::from_le_bytes(
        bytes[4..8]
            .try_into()
            .expect("slice of exactly 4 bytes is always a [u8; 4]"),
    ) as usize;
    let header_end = 8usize
        .checked_add(header_len)
        .context("content asset header length overflows")?;
    if bytes.len() < header_end {
        bail!(
            "content asset truncated: header claims {header_len} bytes, only {} available",
            bytes.len() - 8
        );
    }

    let header: ContentAssetHeader = bincode::deserialize(&bytes[8..header_end])
        .context("failed to deserialize content asset header")?;
    if header.format_version != CONTENT_FORMAT_VERSION {
        bail!(
            "unsupported content asset format version {} (this build expects {CONTENT_FORMAT_VERSION})",
            header.format_version
        );
    }
    Ok((header, &bytes[header_end..]))
}

/// Writes `value` as a content asset at `project_root/address`, creating
/// parent directories as needed.
///
/// `address` is the project-relative path (`"content/hero/body.gasset"`) and
/// is what the asset's id is hashed from; `project_root` is the source tree
/// an editor saves into, which is deliberately *not* the exe-relative
/// runtime root — a save must land in the tree under version control, not
/// beside the binary where the next build overwrites it.
pub fn save_content_asset<A: Asset>(
    value: &A,
    project_root: &std::path::Path,
    address: &str,
) -> anyhow::Result<()> {
    let header = ContentAssetHeader {
        format_version: CONTENT_FORMAT_VERSION,
        asset_id: AssetId::from_path(address),
        references: value.referenced_sub_assets(),
        kind: A::name().to_string(),
    };
    let payload = bincode::serialize(value).context("failed to serialize content asset payload")?;
    let bytes = write_content_asset(&header, &payload)?;

    let path = project_root.join(address);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create '{}'", parent.display()))?;
    }
    std::fs::write(&path, bytes)
        .with_context(|| format!("failed to write content asset '{}'", path.display()))
}
