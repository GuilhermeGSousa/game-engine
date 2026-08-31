use std::path::{Path, PathBuf};

use essential::assets::AssetId;

use crate::{DependencyEntry, ImportContext, ImportError, Importer};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubAssetEntry {
    pub name: String,
    pub asset_id: AssetId,
    pub type_name: String,
    pub references: Vec<AssetId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceIndex {
    pub source_path: PathBuf,
    pub source_hash: u64,
    pub sub_assets: Vec<SubAssetEntry>,
    pub dependencies: Vec<DependencyEntry>,
}

pub struct CookOptions {
    pub manifest_path: PathBuf,
    pub source_root: PathBuf,
    pub output_root: PathBuf,
}

/// The cooked file location for any AssetId is a pure function of the ID
/// alone — no index, no source path needed. This is what lets a nested
/// reference (which only ever carries an AssetId, never a path) resolve to
/// its cooked bytes with no lookup.
pub fn cooked_file_path_for_id(output_root: &Path, id: AssetId) -> PathBuf {
    output_root.join(".cooked").join(format!("{}.bin", id.simple_hex()))
}

pub fn hash_file_contents(path: &Path) -> Result<u64, ImportError> {
    use std::hash::{DefaultHasher, Hash, Hasher};

    let bytes = std::fs::read(path).map_err(|err| ImportError::SourceUnreadable {
        source_path: path.to_path_buf(),
        message: err.to_string(),
    })?;

    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    Ok(hasher.finish())
}

/// Cooks a single source file with the given importer, writing one flat,
/// ID-keyed file per emitted sub-asset under `output_root/.cooked/`.
pub fn cook_source(
    importer: &dyn Importer,
    source_path: &Path,
    relative_source: &Path,
    output_root: &Path,
) -> Result<SourceIndex, ImportError> {
    let source_hash = hash_file_contents(source_path)?;

    let mut ctx = ImportContext::new(relative_source.to_path_buf());
    importer.import(source_path, &mut ctx)?;
    let outputs = ctx.into_parts();

    let cooked_dir = output_root.join(".cooked");
    std::fs::create_dir_all(&cooked_dir).map_err(|err| ImportError::SourceUnreadable {
        source_path: source_path.to_path_buf(),
        message: format!("failed to create cooked output dir: {err}"),
    })?;

    let mut entries = Vec::with_capacity(outputs.sub_assets.len());
    for sub_asset in outputs.sub_assets {
        let cooked_path = cooked_file_path_for_id(output_root, sub_asset.asset_id);
        std::fs::write(&cooked_path, &sub_asset.bytes).map_err(|err| {
            ImportError::SerializationFailed {
                sub_asset_name: sub_asset.name.clone(),
                message: format!("failed to write cooked file: {err}"),
            }
        })?;

        entries.push(SubAssetEntry {
            name: sub_asset.name,
            asset_id: sub_asset.asset_id,
            type_name: sub_asset.type_name.to_string(),
            references: sub_asset.references,
        });
    }

    Ok(SourceIndex {
        source_path: source_path.to_path_buf(),
        source_hash,
        sub_assets: entries,
        dependencies: outputs.dependencies,
    })
}
