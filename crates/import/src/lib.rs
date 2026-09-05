//! The content-asset import driver: runs the offline importers with a
//! content-path sub-asset-id resolver and writes one content asset per
//! emitted sub-asset. The binary (`src/main.rs`) is a thin CLI over
//! [`import_source`].
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context};
use asset_import::{ImportContext, Importer, SubAssetIdResolver};
use essential::assets::content::{
    mint_or_reuse_id, write_content_asset, AssetRegistry, ContentAssetHeader, ImportProvenance,
    CONTENT_FORMAT_VERSION,
};
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

/// One content asset `import_source` wrote, returned so an in-process caller
/// (a future editor) gets structured results instead of parsing addresses
/// back out of printed strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedAsset {
    /// The sub-asset name within the source, e.g. `"mesh/0"`.
    pub sub_asset_name: String,
    /// The project-relative content-tree address it was written to.
    pub address: String,
    /// The asset type tag (`Asset::name()`), e.g. `"Mesh"`.
    pub kind: String,
}

/// Imports one source file into `project_root`, returning the content
/// assets written, and upserts `project_root`'s asset registry so each one
/// is reachable by `AssetServer::load_by_id`.
pub fn import_source(
    source: &Path,
    project_root: &Path,
    config: &ContentConfig,
) -> anyhow::Result<Vec<ImportedAsset>> {
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

    // Identity is minted per address and reused from the header already on
    // disk, so it must be decided once per sub-asset name and shared between
    // the cross-references baked during the importer pass and the headers
    // written afterwards. `content_address` is a pure function of the name,
    // so this memo is the single place an id is decided for this run.
    let minted: Arc<Mutex<HashMap<String, AssetId>>> = Arc::new(Mutex::new(HashMap::new()));

    let owned_source = source.to_path_buf();
    let owned_config = config.clone();
    let owned_root = project_root.to_path_buf();
    let memo = Arc::clone(&minted);
    let resolver: SubAssetIdResolver = Box::new(move |sub_name| {
        let address = content_address(&owned_config, &owned_source, sub_name);
        let mut memo = memo.lock().expect("mint memo poisoned");
        if let Some(id) = memo.get(&address) {
            return *id;
        }
        // A failure to read an existing header would mean a corrupt tree;
        // mint rather than panic in a resolver that cannot return an error,
        // and let the write below surface the real problem.
        let id = mint_or_reuse_id(&owned_root.join(&address)).unwrap_or_else(|_| AssetId::new());
        memo.insert(address, id);
        id
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
            // `<tex>#main` texture id). This phase has no cross-source import
            // story, so warn and keep going rather than aborting the import.
            if !emitted.contains(reference) {
                log::warn!(
                    "'{}' references {reference:?}, which this source does not emit; \
                     leaving it unresolved (cross-source imports land in a later phase)",
                    sub_asset.name
                );
            }
        }

        let address = content_address(config, source, &sub_asset.name);
        let kind = sub_asset.type_name.to_string();
        let header = ContentAssetHeader {
            format_version: CONTENT_FORMAT_VERSION,
            asset_id: sub_asset.asset_id,
            references: sub_asset.references.clone(),
            kind: kind.clone(),
            provenance: Some(ImportProvenance {
                source: source.display().to_string(),
                sub_asset: sub_asset.name.clone(),
            }),
        };
        let bytes = write_content_asset(&header, &sub_asset.bytes)?;

        let path = project_root.join(&address);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create '{}'", parent.display()))?;
        }
        std::fs::write(&path, bytes)
            .with_context(|| format!("failed to write '{}'", path.display()))?;

        written.push(ImportedAsset {
            sub_asset_name: sub_asset.name.clone(),
            address,
            kind,
        });
    }

    // The registry is derived data: rebuilt from the tree rather than merged
    // into, so a content asset that was renamed or moved by hand is
    // re-pointed at where it actually is, and an entry whose file is gone
    // does not linger.
    AssetRegistry::from_content_tree(project_root, &config.root, &config.extension)?
        .save(project_root)?;

    Ok(written)
}
