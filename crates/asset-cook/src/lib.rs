mod cook;
mod import_context;
mod manifest;
mod run;

pub use cook::{
    cook_source, cooked_file_path_for_id, hash_file_contents, CookOptions, SourceIndex,
    SubAssetEntry, COOK_FORMAT_VERSION,
};
pub use import_context::{
    DependencyEntry, EmittedSubAsset, ImportContext, ImportError, ImportOutputs,
};
pub use manifest::{AssetManifest, ManifestEntry};
pub use run::{run_cook, CookReport};

use std::path::{Path, PathBuf};

use essential::assets::AssetId;
use serde::{de::DeserializeOwned, Serialize};

/// A cooked, on-disk representation of one engine asset. Implemented
/// directly on the real asset type wherever possible (e.g. `StandardMaterial`,
/// `Scene`) — a separate DTO is only introduced when the live type holds
/// data that genuinely can't serialize (e.g. GPU descriptor types), never
/// merely because it holds an `AssetHandle<T>` field, since `AssetHandle<T>`
/// is itself serializable.
pub trait CookedAsset: Serialize + DeserializeOwned {
    const TYPE_NAME: &'static str;

    /// AssetIds of every other sub-asset this one references. Used by the
    /// cook tool's global reference-integrity validation pass.
    fn referenced_sub_assets(&self) -> Vec<AssetId> {
        Vec::new()
    }
}

pub trait Importer: Send + Sync {
    fn supported_extensions(&self) -> &'static [&'static str];

    fn import(&self, source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError>;

    fn validate(&self, _sub_assets: &[EmittedSubAsset]) -> Vec<ValidationIssue> {
        Vec::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub message: String,
    pub source_path: PathBuf,
    pub sub_asset_name: Option<String>,
}
