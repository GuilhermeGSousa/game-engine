use std::hash::{DefaultHasher, Hash, Hasher};

pub mod asset_container;
pub mod asset_loader;
pub mod asset_server;
pub mod asset_store;
pub mod handle;
pub mod load_pool;
pub mod utils;

// Path to an asset in a virtual file system.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct AssetPath {
    normalized_path: String,
}

impl AssetPath {
    pub fn new(path: impl AsRef<str>) -> Self {
        let mut normalized = path.as_ref().replace('\\', "/");

        // Remove any leading ./ or .\
        if normalized.starts_with("./") {
            normalized.drain(..2);
        }

        if !normalized.starts_with("res/") {
            normalized = format!("res/{}", normalized);
        }

        // Remove duplicate slashes
        normalized = normalized.replace("//", "/");

        Self {
            normalized_path: normalized,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn to_request_path(&self) -> String {
        self.normalized_path.clone()
    }
}

impl Into<AssetPath> for String {
    fn into(self) -> AssetPath {
        AssetPath::new(self)
    }
}

impl Into<AssetPath> for &str {
    fn into(self) -> AssetPath {
        AssetPath::new(self)
    }
}

impl Into<AssetPath> for &String {
    fn into(self) -> AssetPath {
        AssetPath::new(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(u64);

impl AssetId {
    pub fn new<T: Asset>(path: &AssetPath) -> Self {
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        AssetId(hasher.finish())
    }
}

pub trait Asset: Send + Sync + 'static {
    fn loader() -> Box<dyn asset_loader::AssetLoader<Asset = Self>>;
}
