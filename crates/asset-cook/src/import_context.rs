use std::path::PathBuf;

use essential::assets::AssetId;

use crate::CookedAsset;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DependencyEntry {
    pub path: PathBuf,
    pub content_hash: u64,
}

#[derive(Debug, Clone)]
pub struct EmittedSubAsset {
    pub name: String,
    pub asset_id: AssetId,
    pub type_name: &'static str,
    pub bytes: Vec<u8>,
    pub references: Vec<AssetId>,
}

#[derive(Debug, Clone)]
pub enum ImportError {
    SourceUnreadable { source_path: PathBuf, message: String },
    MalformedSource { source_path: PathBuf, message: String },
    MissingRequiredData { source_path: PathBuf, message: String },
    SerializationFailed { sub_asset_name: String, message: String },
}

#[derive(Debug, Clone)]
pub struct ImportOutputs {
    pub sub_assets: Vec<EmittedSubAsset>,
    pub dependencies: Vec<DependencyEntry>,
}

pub struct ImportContext {
    relative_source: PathBuf,
    sub_assets: Vec<EmittedSubAsset>,
    dependencies: Vec<DependencyEntry>,
}

impl ImportContext {
    pub fn new(relative_source: PathBuf) -> Self {
        Self { relative_source, sub_assets: Vec::new(), dependencies: Vec::new() }
    }

    /// Computes the stable AssetId a sub-asset name resolves to *within this
    /// source file*, without needing to know the final on-disk cooked
    /// layout. Importers use this to build same-file cross-references
    /// (e.g. a material's texture, a scene node's mesh) as real
    /// `AssetHandle::weak(id)` values on the structs they emit.
    pub fn sub_asset_id(&self, name: &str) -> AssetId {
        AssetId::from_path(&format!("{}#{}", self.relative_source.display(), name))
    }

    pub fn emit<T: CookedAsset>(&mut self, name: &str, value: &T) -> Result<(), ImportError> {
        let bytes = bincode::serialize(value).map_err(|err| ImportError::SerializationFailed {
            sub_asset_name: name.to_string(),
            message: err.to_string(),
        })?;

        self.sub_assets.push(EmittedSubAsset {
            name: name.to_string(),
            asset_id: self.sub_asset_id(name),
            type_name: T::TYPE_NAME,
            bytes,
            references: value.referenced_sub_assets(),
        });

        Ok(())
    }

    pub fn track_dependency(&mut self, path: PathBuf, content_hash: u64) {
        self.dependencies.push(DependencyEntry { path, content_hash });
    }

    pub fn into_parts(self) -> ImportOutputs {
        ImportOutputs { sub_assets: self.sub_assets, dependencies: self.dependencies }
    }
}
