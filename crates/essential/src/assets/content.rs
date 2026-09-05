//! Framing for game-ready content assets: `magic | u32 header_len |
//! bincode(ContentAssetHeader) | payload (verbatim)`. The header is read
//! without touching the payload, so a future asset registry can index a
//! whole content tree by scanning headers alone.
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

use super::{Asset, AssetId};

/// Leading bytes of every content asset file.
pub const CONTENT_ASSET_MAGIC: [u8; 4] = *b"GRDY";

/// Bumped whenever `ContentAssetHeader`'s on-disk shape changes
/// incompatibly. `read_content_asset` rejects a mismatch outright.
pub const CONTENT_FORMAT_VERSION: u32 = 2;

/// Where `AssetRegistry` lives, relative to the same root content-asset
/// addresses resolve against (a project root at import/save time, the
/// runtime `ContentAssetRoot` at load time).
pub const REGISTRY_FILE_NAME: &str = "content/.registry.toml";

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
    /// Where this content asset came from, if `import` produced it from a
    /// DCC source rather than an editor saving it directly.
    pub provenance: Option<ImportProvenance>,
}

/// The offline source (and sub-asset within it) that `import` produced a
/// content asset from — lets a future editor show "re-import" provenance
/// without re-deriving it from the address string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportProvenance {
    /// The source path exactly as passed to `import_source`, not otherwise
    /// normalized against any project root.
    pub source: String,
    /// The sub-asset name within that source, e.g. `"mesh/0"`.
    pub sub_asset: String,
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

/// Reads just the header of the content asset at `path`, never the payload.
///
/// A content tree holds whole textures and meshes — tens of megabytes each —
/// so indexing one by reading every file whole is not viable. This reads the
/// 8-byte magic-and-length prefix, then exactly `header_len` more bytes.
pub fn read_content_asset_header(path: &Path) -> anyhow::Result<ContentAssetHeader> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)
        .with_context(|| format!("failed to open '{}'", path.display()))?;

    let mut prefix = [0u8; 8];
    file.read_exact(&mut prefix)
        .with_context(|| format!("'{}' is too short to be a content asset", path.display()))?;
    if prefix[..4] != CONTENT_ASSET_MAGIC {
        bail!(
            "not a content asset: '{}' is missing the GRDY magic prefix",
            path.display()
        );
    }

    let header_len = u32::from_le_bytes(
        prefix[4..8]
            .try_into()
            .expect("slice of exactly 4 bytes is always a [u8; 4]"),
    ) as usize;

    let mut header_bytes = vec![0u8; header_len];
    file.read_exact(&mut header_bytes).with_context(|| {
        format!(
            "content asset '{}' truncated: header claims {header_len} bytes",
            path.display()
        )
    })?;

    let header: ContentAssetHeader = bincode::deserialize(&header_bytes)
        .with_context(|| format!("failed to deserialize header of '{}'", path.display()))?;
    if header.format_version != CONTENT_FORMAT_VERSION {
        bail!(
            "unsupported content asset format version {} in '{}' (this build expects {CONTENT_FORMAT_VERSION})",
            header.format_version,
            path.display()
        );
    }
    Ok(header)
}

/// Writes `value` as a content asset at `project_root/address`, creating
/// parent directories as needed, and upserts the asset registry so a
/// path-less (`AssetServer::load_by_id`) load can find it later.
///
/// The asset's id is *minted* the first time an address is written and
/// reused every time after, by reading the header already on disk — so a
/// re-save keeps the identity that existing references name. `address` is
/// the project-relative path (`"content/hero/body.gasset"`); `project_root`
/// is the source tree an editor saves into, which is deliberately *not* the
/// exe-relative runtime root — a save must land in the tree under version
/// control, not beside the binary where the next build overwrites it.
pub fn save_content_asset<A: Asset>(
    value: &A,
    project_root: &Path,
    address: &str,
) -> anyhow::Result<()> {
    let path = project_root.join(address);
    let asset_id = mint_or_reuse_id(&path)?;
    let header = ContentAssetHeader {
        format_version: CONTENT_FORMAT_VERSION,
        asset_id,
        references: value.referenced_sub_assets(),
        kind: A::name().to_string(),
        provenance: None,
    };
    let payload = bincode::serialize(value).context("failed to serialize content asset payload")?;
    let bytes = write_content_asset(&header, &payload)?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create '{}'", parent.display()))?;
    }
    std::fs::write(&path, bytes)
        .with_context(|| format!("failed to write content asset '{}'", path.display()))?;

    let mut registry = AssetRegistry::load(project_root)?;
    registry.insert(asset_id, address);
    registry.save(project_root)
}

/// The id to write at `path`: the one already in the file's header if a
/// content asset is there, otherwise a freshly minted one. Identity is
/// assigned once and then belongs to the asset, not to its location.
pub fn mint_or_reuse_id(path: &Path) -> anyhow::Result<AssetId> {
    if path.exists() {
        return Ok(read_content_asset_header(path)?.asset_id);
    }
    Ok(AssetId::new())
}

/// A serialized `[assets]` table in `.registry.toml`: `AssetId::simple_hex()`
/// keys to content-tree address values.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RegistryFile {
    #[serde(default)]
    assets: BTreeMap<String, String>,
}

/// Maps AssetIds to their content-tree address. Backs `AssetServer`'s
/// path-less (`load_by_id`) resolution now that content assets live at
/// literal human paths instead of the old `.cooked/<hex>.bin`
/// naming-is-lookup convention: `import` and `save_content_asset` upsert it
/// directly, merge-only, never pruned.
#[derive(Debug, Clone, Default)]
pub struct AssetRegistry {
    entries: BTreeMap<AssetId, String>,
    by_address: HashMap<String, AssetId>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads `<project_root>/content/.registry.toml`, or an empty registry
    /// if it does not exist yet (a content tree with nothing imported or
    /// saved into it has no registry file).
    pub fn load(project_root: &Path) -> anyhow::Result<Self> {
        let path = project_root.join(REGISTRY_FILE_NAME);
        match std::fs::read_to_string(&path) {
            Ok(text) => Self::parse(&text),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err).with_context(|| format!("failed to read '{}'", path.display())),
        }
    }

    pub fn parse(text: &str) -> anyhow::Result<Self> {
        let file: RegistryFile = toml::from_str(text).context("failed to parse asset registry")?;
        let mut registry = Self::default();
        for (hex, address) in file.assets {
            let id = AssetId::from_simple_hex(&hex)
                .map_err(|err| anyhow::anyhow!("invalid asset id '{hex}' in registry: {err}"))?;
            if let Some(existing) = registry.by_address.get(&address) {
                bail!(
                    "asset registry maps two ids ({} and {}) to the same address '{address}'",
                    existing.simple_hex(),
                    id.simple_hex()
                );
            }
            registry.insert(id, address);
        }
        Ok(registry)
    }

    /// Builds a registry by scanning `<project_root>/<content_root>` for
    /// `*.<extension>` files and reading each one's header.
    ///
    /// This is the authoritative way to produce a registry: identity lives in
    /// the header, so a scan re-points an id at wherever its file actually is
    /// now — which is what lets a content asset be renamed or moved without
    /// breaking the references that name its id. An absent content tree is an
    /// empty registry, not an error.
    pub fn from_content_tree(
        project_root: &Path,
        content_root: &str,
        extension: &str,
    ) -> anyhow::Result<Self> {
        let root = project_root.join(content_root);
        let mut registry = Self::default();
        let mut source_of: HashMap<AssetId, String> = HashMap::new();

        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(err) => {
                    return Err(err).with_context(|| format!("failed to read '{}'", dir.display()))
                }
            };
            for entry in entries {
                let path = entry
                    .with_context(|| format!("failed to read an entry of '{}'", dir.display()))?
                    .path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|e| e.to_str()) != Some(extension) {
                    continue;
                }

                let header = read_content_asset_header(&path)?;
                let address = path
                    .strip_prefix(project_root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");

                if let Some(previous) = source_of.get(&header.asset_id) {
                    bail!(
                        "content tree is malformed: '{previous}' and '{address}' both carry asset id {}",
                        header.asset_id.simple_hex()
                    );
                }
                source_of.insert(header.asset_id, address.clone());
                registry.insert(header.asset_id, address);
            }
        }

        Ok(registry)
    }

    pub fn save(&self, project_root: &Path) -> anyhow::Result<()> {
        let path = project_root.join(REGISTRY_FILE_NAME);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create '{}'", parent.display()))?;
        }
        let assets = self
            .entries
            .iter()
            .map(|(id, address)| (id.simple_hex(), address.clone()))
            .collect();
        let text = toml::to_string_pretty(&RegistryFile { assets })
            .context("failed to serialize asset registry")?;
        std::fs::write(&path, text).with_context(|| format!("failed to write '{}'", path.display()))
    }

    pub fn get(&self, id: AssetId) -> Option<&str> {
        self.entries.get(&id).map(String::as_str)
    }

    pub fn insert(&mut self, id: AssetId, address: impl Into<String>) {
        let address = address.into();
        if let Some(previous) = self.entries.insert(id, address.clone()) {
            self.by_address.remove(&previous);
        }
        self.by_address.insert(address, id);
    }

    pub fn remove(&mut self, id: AssetId) -> Option<String> {
        let address = self.entries.remove(&id)?;
        self.by_address.remove(&address);
        Some(address)
    }

    /// The id of the asset at `address`, for a path-based load. The inverse
    /// of [`AssetRegistry::get`].
    pub fn id_for_address(&self, address: &str) -> Option<AssetId> {
        self.by_address.get(address).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (AssetId, &str)> {
        self.entries
            .iter()
            .map(|(id, address)| (*id, address.as_str()))
    }
}
