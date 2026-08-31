//! Covers incremental cook skip logic: unchanged sources (and their tracked
//! dependencies) are not re-imported on a second cook run.
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use asset_cook::{run_cook, CookOptions, CookedAsset, ImportContext, ImportError, Importer};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct CountedThing {
    value: u32,
}

impl CookedAsset for CountedThing {
    const TYPE_NAME: &'static str = "CountedThing";
}

struct CountingImporter {
    import_count: AtomicUsize,
}

impl Importer for CountingImporter {
    fn supported_extensions(&self) -> &'static [&'static str] {
        &["fake"]
    }

    fn import(&self, source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError> {
        self.import_count.fetch_add(1, Ordering::SeqCst);
        let dep_path = source_path.with_extension("dep");
        let dep_hash = asset_cook::hash_file_contents(&dep_path)?;
        ctx.track_dependency(dep_path, dep_hash);
        ctx.emit("thing/0", &CountedThing { value: 1 }).unwrap();
        Ok(())
    }
}

fn setup() -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let temp_dir = std::env::temp_dir().join(format!("asset-cook-incremental-{}", std::process::id()));
    let source_root = temp_dir.join("assets");
    let output_root = temp_dir.join("res");
    std::fs::create_dir_all(&source_root).unwrap();
    std::fs::write(source_root.join("thing.fake"), b"source v1").unwrap();
    std::fs::write(source_root.join("thing.dep"), b"dep v1").unwrap();

    let manifest_path = temp_dir.join("assets.toml");
    std::fs::write(&manifest_path, "[[assets]]\npath = \"thing.fake\"\n").unwrap();

    (manifest_path, source_root, output_root)
}

#[test]
fn second_cook_skips_unchanged_source_and_dependency() {
    let (manifest_path, source_root, output_root) = setup();
    let importers: Vec<Box<dyn Importer>> = vec![Box::new(CountingImporter { import_count: AtomicUsize::new(0) })];
    let options = CookOptions { manifest_path, source_root: source_root.clone(), output_root };

    let first = run_cook(&importers, &options);
    assert_eq!(first.errors.len(), 0, "first cook must succeed: {:?}", first.errors);
    assert_eq!(first.cooked.len(), 1);

    let second = run_cook(&importers, &options);
    assert_eq!(second.errors.len(), 0);
    assert_eq!(second.cooked.len(), 0, "nothing changed, so nothing should be re-cooked");
    assert_eq!(second.skipped.len(), 1);

    std::fs::remove_dir_all(source_root.parent().unwrap()).ok();
}

#[test]
fn changing_a_tracked_dependency_forces_reimport() {
    let (manifest_path, source_root, output_root) = setup();
    let importers: Vec<Box<dyn Importer>> = vec![Box::new(CountingImporter { import_count: AtomicUsize::new(0) })];
    let options = CookOptions { manifest_path, source_root: source_root.clone(), output_root };

    run_cook(&importers, &options);
    std::fs::write(source_root.join("thing.dep"), b"dep v2 - changed").unwrap();
    let second = run_cook(&importers, &options);

    assert_eq!(second.cooked.len(), 1, "a changed dependency must force the source to re-import");
    assert_eq!(second.skipped.len(), 0);

    std::fs::remove_dir_all(source_root.parent().unwrap()).ok();
}
