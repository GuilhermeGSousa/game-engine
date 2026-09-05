use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Deserialize;

/// `content.toml` at a project root. Only the `import` side reads this —
/// the runtime gets the extension from the asset path and the root from
/// `ContentAssetRoot`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ContentConfig {
    /// Uniform extension for every content asset, without the dot.
    pub extension: String,
    /// Content tree root, relative to the project root.
    pub root: String,
}

impl Default for ContentConfig {
    fn default() -> Self {
        Self {
            extension: "gasset".to_string(),
            root: "content".to_string(),
        }
    }
}

impl ContentConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read '{}'", path.display()))?;
        toml::from_str(&text).with_context(|| format!("failed to parse '{}'", path.display()))
    }

    /// Loads `content.toml` from `dir` if present, else the defaults.
    pub fn load_or_default(dir: &Path) -> anyhow::Result<Self> {
        let path = dir.join("content.toml");
        if path.exists() {
            Self::load(&path)
        } else {
            Ok(Self::default())
        }
    }
}

/// `<content-root>/<source-stem>/<sanitized-sub>.<ext>`, the project-relative
/// address a sub-asset is written to and referenced by.
pub fn content_address(config: &ContentConfig, source: &Path, sub_name: &str) -> String {
    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "asset".to_string());
    // A '#' is fragile in an address that doubles as a filesystem path and a
    // cross-reference string (it reads as a fragment separator in URL-shaped
    // contexts, and needs escaping in others); '\\' would split the join on
    // Windows.
    let sanitized = sub_name.replace(['/', '\\', '#'], "_");
    format!("{}/{stem}/{sanitized}.{}", config.root, config.extension)
}

/// The project root a config path implies (its containing directory).
pub fn project_root_of(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_partial_config_fills_the_missing_field_from_default() {
        let config: ContentConfig = toml::from_str(r#"root = "content""#).expect("parses");
        assert_eq!(config.root, "content");
        assert_eq!(
            config.extension, "gasset",
            "the unset field takes its default"
        );
    }

    #[test]
    fn a_hash_in_a_sub_asset_name_is_sanitized_out_of_the_address() {
        let address = content_address(
            &ContentConfig::default(),
            Path::new("hero.obj"),
            "material/steel#main",
        );
        assert!(!address.contains('#'), "got: {address}");
    }
}
