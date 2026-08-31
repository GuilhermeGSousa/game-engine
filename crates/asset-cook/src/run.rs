use std::path::PathBuf;

use crate::{
    cook_source, cooked_file_path_for_id, hash_file_contents, AssetManifest, CookOptions,
    ImportError, Importer, SourceIndex,
};

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
    let mut all_indices: Vec<SourceIndex> = Vec::new();

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

        let extension = source_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default();
        let Some(importer) = importers
            .iter()
            .find(|i| i.supported_extensions().contains(&extension))
        else {
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
                    all_indices.push(existing_index);
                    continue;
                }
            }
        }

        match cook_source(
            importer.as_ref(),
            &source_path,
            &relative_source,
            &options.output_root,
        ) {
            Ok(index) => {
                if let Err(err) = std::fs::create_dir_all(index_path.parent().unwrap()) {
                    eprintln!(
                        "warning: failed to create incremental index dir for {}: {err}",
                        relative_source.display()
                    );
                }
                match bincode::serialize(&index) {
                    Ok(bytes) => {
                        if let Err(err) = std::fs::write(&index_path, bytes) {
                            eprintln!(
                                "warning: failed to persist incremental index for {}: {err}",
                                relative_source.display()
                            );
                        }
                    }
                    Err(err) => {
                        eprintln!(
                            "warning: failed to serialize incremental index for {}: {err}",
                            relative_source.display()
                        );
                    }
                }

                let emitted: Vec<crate::EmittedSubAsset> = index
                    .sub_assets
                    .iter()
                    .map(|entry| crate::EmittedSubAsset {
                        name: entry.name.clone(),
                        asset_id: entry.asset_id,
                        type_name: Box::leak(entry.type_name.clone().into_boxed_str()),
                        bytes: std::fs::read(cooked_file_path_for_id(
                            &options.output_root,
                            entry.asset_id,
                        ))
                        .unwrap_or_default(),
                        references: entry.references.clone(),
                    })
                    .collect();

                for issue in importer.validate(&emitted) {
                    if issue.severity == crate::ValidationSeverity::Error {
                        report.errors.push(ImportError::MissingRequiredData {
                            source_path: issue.source_path.clone(),
                            message: issue.message.clone(),
                        });
                    } else {
                        log::warn!(
                            "validation warning for '{}' ({:?}): {}",
                            issue.source_path.display(),
                            issue.sub_asset_name,
                            issue.message
                        );
                    }
                }

                report.cooked.push(source_path);
                all_indices.push(index);
            }
            Err(err) => report.errors.push(err),
        }
    }

    let produced: std::collections::HashSet<essential::assets::AssetId> = all_indices
        .iter()
        .flat_map(|index| index.sub_assets.iter().map(|s| s.asset_id))
        .collect();

    for index in &all_indices {
        for sub_asset in &index.sub_assets {
            for reference in &sub_asset.references {
                if !produced.contains(reference) {
                    report.errors.push(ImportError::MissingRequiredData {
                        source_path: index.source_path.clone(),
                        message: format!(
                            "'{}' references AssetId {:?}, which was never produced",
                            sub_asset.name, reference
                        ),
                    });
                }
            }
        }
    }

    report
}
