use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ManifestEntry {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct AssetManifest {
    pub assets: Vec<ManifestEntry>,
}

impl AssetManifest {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }
}
