use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;

/// `essential::assets::content::REGISTRY_FILE_NAME`, restated rather than
/// imported: `essential` is a `cdylib` whose entire dependency tree would
/// have to be rebuilt for the host to be usable as a build dependency,
/// which collides on `libessential.so` in the shared `deps/` directory.
const REGISTRY_FILE_NAME: &str = "content/.registry.toml";

/// Copies the `content/` directory next to the built binary, so the
/// executable-relative `ContentAssetRoot::Directory` default finds it.
///
/// Every example in this workspace builds into the same shared `target/`
/// directory, so `<output_path>/content/` is shared too. The `.gasset` files
/// live under distinct per-example subdirectories and never collide, but
/// `content/.registry.toml` is a single fixed path: copying it verbatim
/// would let whichever example built last erase every other example's
/// entries, breaking their `load_by_id` calls.
///
/// So each example rewrites the registry as the union of *every* example's
/// committed registry rather than merging whatever it happens to find at the
/// destination. Cargo runs sibling build scripts in parallel, so a
/// read-modify-write of the shared file would race; a union computed from
/// the committed source trees — which don't change during a build — makes
/// every build script write byte-identical content, after its own copy, so
/// the last write always leaves the complete registry no matter how the
/// scripts interleave. Entries for an example that isn't built are inert:
/// only that example ever loads those ids.
fn main() -> anyhow::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).canonicalize()?;
    let content_path = manifest_dir.join("content");
    println!("cargo:rerun-if-changed={}", content_path.display());

    if !content_path.exists() {
        return Ok(());
    }

    // OUT_DIR is target/<profile>/build/<pkg>-<hash>/out, so the profile
    // directory — where the binary lands — is three levels up.
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let output_path = out_dir.ancestors().nth(3).unwrap().to_path_buf();

    copy_items(
        &[content_path],
        Path::new(&output_path),
        &CopyOptions {
            overwrite: true,
            ..Default::default()
        },
    )?;

    let merged = union_of_sibling_registries(&manifest_dir)?;
    save_registry(&output_path.join(REGISTRY_FILE_NAME), &merged)?;

    Ok(())
}

/// Every registry under `<examples>/*/content/`, including this example's
/// own, keyed by `AssetId` hex. Sibling directories are visited in sorted
/// order so the result is identical whichever example computes it.
fn union_of_sibling_registries(manifest_dir: &Path) -> anyhow::Result<BTreeMap<String, String>> {
    let Some(examples_dir) = manifest_dir.parent() else {
        return Ok(BTreeMap::new());
    };

    let mut siblings: Vec<PathBuf> = std::fs::read_dir(examples_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    siblings.sort();

    let mut merged = BTreeMap::new();
    for sibling in siblings {
        for (id, address) in load_registry(&sibling.join(REGISTRY_FILE_NAME)) {
            merged.entry(id).or_insert(address);
        }
    }
    Ok(merged)
}

/// Reads the `[assets]` table of a `.registry.toml` as `AssetId` hex to
/// content-tree address. A missing or unreadable registry is an empty one.
fn load_registry(path: &Path) -> BTreeMap<String, String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let Ok(table) = text.parse::<toml::Table>() else {
        return BTreeMap::new();
    };
    table
        .get("assets")
        .and_then(toml::Value::as_table)
        .map(|assets| {
            assets
                .iter()
                .filter_map(|(id, address)| address.as_str().map(|a| (id.clone(), a.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

fn save_registry(path: &Path, entries: &BTreeMap<String, String>) -> anyhow::Result<()> {
    let assets = entries
        .iter()
        .map(|(id, address)| (id.clone(), toml::Value::String(address.clone())))
        .collect();
    let mut root = toml::Table::new();
    root.insert("assets".to_string(), toml::Value::Table(assets));

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, toml::to_string_pretty(&root)?)?;
    Ok(())
}
