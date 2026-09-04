mod cook;
mod import_context;
mod manifest;
mod run;

pub use cook::{
    cook_source, cooked_file_path_for_id, hash_file_contents, CookOptions, SourceIndex,
    SubAssetEntry, COOK_FORMAT_VERSION,
};
pub use import_context::{
    DependencyEntry, EmittedSubAsset, ImportContext, ImportError, ImportOutputs, SubAssetIdResolver,
};
pub use manifest::{AssetManifest, ManifestEntry};
pub use run::{run_cook, CookReport};

use std::path::{Path, PathBuf};

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
