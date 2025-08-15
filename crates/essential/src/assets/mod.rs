use std::{
    borrow::Cow,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

pub mod asset_container;
pub mod asset_loader;
pub mod asset_server;
pub mod asset_store;
pub mod handle;
pub mod load_pool;
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

        if !normalized.starts_with("res/") {
            normalized = format!("res/{}", normalized);
        }

        AssetPath {
            normalized_path: Cow::Owned(Path::new(&normalized).to_owned()),
        }
    }

    pub fn to_path(&self) -> &Path {
        &self.normalized_path
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
    type UsageSettings: Send + Sync;
    fn loader() -> Box<dyn asset_loader::AssetLoader<Asset = Self>>;

    fn default_usage_settings() -> Self::UsageSettings;
}
