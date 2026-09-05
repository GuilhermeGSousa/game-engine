use std::{
    borrow::Cow,
    hash::Hash,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use essential_macros::Asset;

pub mod asset_container;
pub mod asset_loader;
pub mod asset_server;
pub mod asset_store;
pub mod content;
pub mod handle;
pub mod utils;

// Path to an asset in a virtual file system.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct AssetPath<'a> {
    normalized_path: Cow<'a, Path>,
}

impl<'a> AssetPath<'a> {
    pub fn new(path: impl AsRef<str>) -> Self {
        let mut normalized = path.as_ref().replace('\\', "/");

        // Remove any leading ./ or .\
        if normalized.starts_with("./") {
            normalized.drain(..2);
        }

        AssetPath {
            normalized_path: Cow::Owned(Path::new(&normalized).to_owned()),
        }
    }

    pub fn to_path(&self) -> &Path {
        &self.normalized_path
    }

    /// Recovers the logical asset address this path was constructed from —
    /// the string a caller wrote in a `load()` call or that `import`'s
    /// content-address convention produced (e.g.
    /// `"content/hero/scene.gasset"`) — as an owned `String`.
    ///
    /// This is what `AssetId::from_path` must be given so that a runtime
    /// `load()` call and import-time ID computation (which both derive an
    /// `AssetId` from the same address string) agree on the same `AssetId`
    /// for the same asset.
    pub fn address(&self) -> String {
        self.normalized_path.to_string_lossy().into_owned()
    }

    pub fn into_owned(self) -> AssetPath<'static> {
        AssetPath {
            normalized_path: Cow::Owned(self.normalized_path.into_owned()),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn to_string(&self) -> &str {
        self.normalized_path.to_str().unwrap()
    }
}

impl<'a> From<PathBuf> for AssetPath<'a> {
    fn from(path: PathBuf) -> Self {
        AssetPath {
            normalized_path: Cow::Owned(path),
        }
    }
}

impl<'a> From<String> for AssetPath<'a> {
    fn from(path: String) -> Self {
        AssetPath::new(path)
    }
}

impl<'a> From<&'a str> for AssetPath<'a> {
    fn from(path: &'a str) -> Self {
        AssetPath::new(path)
    }
}

/// Where the runtime finds content asset files. Only the root differs per
/// platform; every address is a full path relative to it (e.g.
/// `"content/hero/scene.gasset"` — the `content/` segment is part of the
/// address, not injected by the root).
#[derive(Debug, Clone)]
pub enum ContentAssetRoot {
    /// Native: the directory containing the executable.
    Directory(PathBuf),
    /// wasm: the page origin, e.g. `"http://host"`.
    UrlBase(String),
}

impl ContentAssetRoot {
    /// Native: `<directory containing the executable>`, matching what each
    /// example's `build.rs` copies its `content/` tree next to. wasm:
    /// `<page origin>`, matching Trunk's `copy-dir` target.
    pub fn default_for_platform() -> Self {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let origin = web_sys::window()
                    .and_then(|window| window.location().origin().ok())
                    .unwrap_or_default();
                ContentAssetRoot::UrlBase(origin)
            } else {
                let exe_dir = std::env::current_exe()
                    .ok()
                    .and_then(|exe| exe.parent().map(Path::to_path_buf))
                    .unwrap_or_else(|| PathBuf::from("."));
                ContentAssetRoot::Directory(exe_dir)
            }
        }
    }
}

impl Default for ContentAssetRoot {
    fn default() -> Self {
        Self::default_for_platform()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssetId(Uuid);

/// Fixed namespace for deriving AssetIds from asset paths, so the same
/// path string always hashes to the same UUID (v5) regardless of process
/// or machine. Generated once via `uuid::Uuid::new_v4()` and hard-coded —
/// it must never change once assets have been addressed with it.
const ASSET_PATH_NAMESPACE: Uuid = Uuid::from_bytes([
    0x6d, 0x1a, 0x9a, 0x3e, 0x2f, 0x0b, 0x4a, 0x77, 0x8e, 0x92, 0x1a, 0x64, 0xaf, 0x03, 0x5c, 0x11,
]);

impl AssetId {
    pub fn new() -> Self {
        AssetId(Uuid::new_v4())
    }

    /// Deterministically derives a stable AssetId from a full asset address
    /// string (e.g. "models/character.gltf#texture/albedo"). The same
    /// string always produces the same ID, with no shared state required —
    /// this is what lets the import tool and the runtime independently agree
    /// on an asset's identity.
    pub fn from_path(path: &str) -> Self {
        AssetId(Uuid::new_v5(&ASSET_PATH_NAMESPACE, path.as_bytes()))
    }

    /// The AssetId's underlying UUID as 32 lowercase hex digits, no hyphens —
    /// used as the `AssetRegistry` key (a text-friendly, collision-free pure
    /// function of the ID).
    pub fn simple_hex(&self) -> String {
        self.0.simple().to_string()
    }

    /// Inverse of `simple_hex()` — parses an AssetId back from its
    /// 32-lowercase-hex-digit form. Used by `AssetRegistry` to reconstruct
    /// ids read back from `.registry.toml`.
    pub fn from_simple_hex(hex: &str) -> Result<Self, uuid::Error> {
        Uuid::parse_str(hex).map(AssetId)
    }
}

impl Default for AssetId {
    fn default() -> Self {
        Self::new()
    }
}

/// A unit of loadable engine content. The `Serialize + DeserializeOwned`
/// supertrait means a serialized, on-disk asset is just an `Asset` —
/// `ImportContext::emit` needs no separate DTO trait. Reach for a distinct
/// DTO type only when the live asset holds data that genuinely cannot
/// serialize (GPU descriptor handles, `&'static` refs) — never merely for an
/// `AssetHandle<T>` field, which serializes to its bare `AssetId`.
pub trait Asset: Send + Sync + 'static + serde::Serialize + serde::de::DeserializeOwned {
    fn name() -> &'static str;

    /// AssetIds of every sub-asset this one references — the import tool's
    /// reference-integrity pass. Empty for leaf assets.
    fn referenced_sub_assets(&self) -> Vec<AssetId> {
        Vec::new()
    }
}

pub trait LoadableAsset: Asset {
    type UsageSettings: Send + Sync;
    fn loader() -> Box<dyn asset_loader::AssetLoader<Asset = Self>>;

    fn default_usage_settings() -> Self::UsageSettings;
}
