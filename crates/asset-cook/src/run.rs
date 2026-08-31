use std::path::PathBuf;

use crate::{cook_source, hash_file_contents, AssetManifest, CookOptions, ImportError, Importer, SourceIndex};

#[derive(Debug, Default)]
pub struct CookReport {
    pub cooked: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
    pub errors: Vec<ImportError>,
}

fn index_path_for(output_root: &std::path::Path, relative_source: &std::path::Path) -> PathBuf {
    let flattened = relative_source.to_string_lossy().replace(['/', '\\'], "_");
    output_root.join(".index").join(format!("{flattened}.bin"))
}

fn source_is_unchanged(existing: &SourceIndex, source_path: &std::path::Path) -> bool {
    let Ok(current_source_hash) = hash_file_contents(source_path) else {
        return false;
    };
    if current_source_hash != existing.source_hash {
        return false;
    }

    existing.dependencies.iter().all(|dependency| {
        matches!(hash_file_contents(&dependency.path), Ok(hash) if hash == dependency.content_hash)
    })
}

pub fn run_cook(importers: &[Box<dyn Importer>], options: &CookOptions) -> CookReport {
    let mut report = CookReport::default();

    let manifest = match AssetManifest::load(&options.manifest_path) {
        Ok(manifest) => manifest,
        Err(err) => {
            report.errors.push(ImportError::SourceUnreadable {
                source_path: options.manifest_path.clone(),
                message: err.to_string(),
            });
            return report;
        }
    };

    for entry in manifest.assets {
        let relative_source = PathBuf::from(&entry.path);
        let source_path = options.source_root.join(&relative_source);

        let extension = source_path.extension().and_then(|ext| ext.to_str()).unwrap_or_default();
        let Some(importer) = importers.iter().find(|i| i.supported_extensions().contains(&extension)) else {
            report.errors.push(ImportError::MalformedSource {
                source_path: source_path.clone(),
                message: format!("no importer registered for extension '{extension}'"),
            });
            continue;
        };

        let index_path = index_path_for(&options.output_root, &relative_source);
        if let Ok(existing_bytes) = std::fs::read(&index_path) {
            if let Ok(existing_index) = bincode::deserialize::<SourceIndex>(&existing_bytes) {
                if source_is_unchanged(&existing_index, &source_path) {
                    report.skipped.push(source_path);
                    continue;
                }
            }
        }

        match cook_source(importer.as_ref(), &source_path, &relative_source, &options.output_root) {
            Ok(index) => {
                std::fs::create_dir_all(index_path.parent().unwrap()).ok();
                if let Ok(bytes) = bincode::serialize(&index) {
                    let _ = std::fs::write(&index_path, bytes);
                }
                report.cooked.push(source_path);
            }
            Err(err) => report.errors.push(err),
        }
    }

    report
}
