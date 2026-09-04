//! The content-asset import driver: runs the offline importers with a
//! content-path sub-asset-id resolver and writes one content asset per
//! emitted sub-asset. The binary (`src/main.rs`) is a thin CLI over
//! [`import_source`].
use std::collections::HashSet;
use std::path::Path;

use anyhow::{bail, Context};
use asset_import::{ImportContext, Importer, SubAssetIdResolver};
use essential::assets::content::{write_content_asset, ContentAssetHeader, CONTENT_FORMAT_VERSION};
use essential::assets::AssetId;

pub mod config;

use config::{content_address, ContentConfig};

fn registered_importers() -> Vec<Box<dyn Importer>> {
    vec![
        Box::new(render::importers::image_importer::ImageImporter),
        Box::new(gltf_loader::gltf_importer::GltfImporter),
        Box::new(obj_loader::obj_importer::ObjImporter),
    ]
}

/// Imports one source file into `project_root`, returning the
/// project-relative addresses written.
pub fn import_source(
    source: &Path,
    project_root: &Path,
    config: &ContentConfig,
) -> anyhow::Result<Vec<String>> {
    let extension = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let importers = registered_importers();
    let Some(importer) = importers
        .iter()
        .find(|i| i.supported_extensions().contains(&extension.as_str()))
    else {
        bail!(
            "no importer handles '.{extension}' (source '{}')",
            source.display()
        );
    };

    // Cross-references resolve to content-tree addresses. `content_address`
    // is a pure function of the sub-asset name, so one importer pass is
    // enough — no need to discover the sub-asset set first.
    let owned_source = source.to_path_buf();
    let owned_config = config.clone();
    let resolver: SubAssetIdResolver = Box::new(move |sub_name| {
        AssetId::from_path(&content_address(&owned_config, &owned_source, sub_name))
    });

    let mut ctx = ImportContext::with_sub_asset_id_resolver(source.to_path_buf(), resolver);
    importer
        .import(source, &mut ctx)
        .map_err(|err| anyhow::anyhow!("{err:?}"))
        .with_context(|| format!("importing '{}'", source.display()))?;
    let outputs = ctx.into_parts();

    let emitted: HashSet<AssetId> = outputs.sub_assets.iter().map(|s| s.asset_id).collect();
    let mut written = Vec::with_capacity(outputs.sub_assets.len());

    for sub_asset in &outputs.sub_assets {
        for reference in &sub_asset.references {
            // A reference outside this source's emitted set is a cross-source
            // link the content-path resolver never produced (e.g. an OBJ's
            // `<tex>#main` texture id). Phase 1 has no cross-source import
            // story, so warn and keep going rather than aborting the import.
            if !emitted.contains(reference) {
                log::warn!(
                    "'{}' references {reference:?}, which this source does not emit; \
                     leaving it unresolved (cross-source imports land in Phase 2)",
                    sub_asset.name
                );
            }
        }

        let address = content_address(config, source, &sub_asset.name);
        let header = ContentAssetHeader {
            format_version: CONTENT_FORMAT_VERSION,
            asset_id: sub_asset.asset_id,
            references: sub_asset.references.clone(),
            kind: sub_asset.type_name.to_string(),
        };
        let bytes = write_content_asset(&header, &sub_asset.bytes)?;

        let path = project_root.join(&address);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create '{}'", parent.display()))?;
        }
        std::fs::write(&path, bytes)
            .with_context(|| format!("failed to write '{}'", path.display()))?;
        written.push(address);
    }

    Ok(written)
}
