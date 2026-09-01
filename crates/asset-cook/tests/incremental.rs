//! Covers incremental cook skip logic: unchanged sources (and their tracked
//! dependencies) are not re-imported on a second cook run.
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use asset_cook::{
    run_cook, CookOptions, CookedAsset, DependencyEntry, ImportContext, ImportError, Importer,
    SourceIndex,
};
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

struct Fixture {
    manifest_path: std::path::PathBuf,
    source_root: std::path::PathBuf,
    output_root: std::path::PathBuf,
}

fn setup(name: &str) -> Fixture {
    let temp_dir = std::env::temp_dir().join(format!(
        "asset-cook-incremental-{}-{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let source_root = temp_dir.join("assets");
    let output_root = temp_dir.join("res");
    std::fs::create_dir_all(&source_root).unwrap();
    std::fs::write(source_root.join("thing.fake"), b"source v1").unwrap();
    std::fs::write(source_root.join("thing.dep"), b"dep v1").unwrap();
    let manifest_path = temp_dir.join("assets.toml");
    std::fs::write(&manifest_path, "[[assets]]\npath = \"thing.fake\"\n").unwrap();
    Fixture {
        manifest_path,
        source_root,
        output_root,
    }
}

#[test]
fn second_cook_skips_unchanged_source_and_dependency() {
    let fx = setup("skips_unchanged");
    let importers: Vec<Box<dyn Importer>> = vec![Box::new(CountingImporter {
        import_count: AtomicUsize::new(0),
    })];
    let options = CookOptions {
        manifest_path: fx.manifest_path,
        source_root: fx.source_root.clone(),
        output_root: fx.output_root,
    };

    let first = run_cook(&importers, &options);
    assert_eq!(
        first.errors.len(),
        0,
        "first cook must succeed: {:?}",
        first.errors
    );
    assert_eq!(
        first.cooked.len(),
        1,
        "first cook must cook the one manifest entry"
    );

    let second = run_cook(&importers, &options);
    assert_eq!(second.errors.len(), 0, "second cook must have no errors");
    assert_eq!(
        second.cooked.len(),
        0,
        "nothing changed, so nothing should be re-cooked"
    );
    assert_eq!(second.skipped.len(), 1, "unchanged source must be skipped");
}

#[test]
fn changing_a_tracked_dependency_forces_reimport() {
    let fx = setup("dep_change_forces_reimport");
    let importers: Vec<Box<dyn Importer>> = vec![Box::new(CountingImporter {
        import_count: AtomicUsize::new(0),
    })];
    let options = CookOptions {
        manifest_path: fx.manifest_path,
        source_root: fx.source_root.clone(),
        output_root: fx.output_root,
    };

    run_cook(&importers, &options);
    std::fs::write(fx.source_root.join("thing.dep"), b"dep v2 - changed").unwrap();
    let second = run_cook(&importers, &options);

    assert_eq!(
        second.cooked.len(),
        1,
        "a changed dependency must force the source to re-import"
    );
    assert_eq!(
        second.skipped.len(),
        0,
        "no sources should be skipped when dependency changed"
    );
}

#[test]
fn stale_cook_format_version_forces_reimport() {
    let fx = setup("format_version_change_forces_reimport");
    let importers: Vec<Box<dyn Importer>> = vec![Box::new(CountingImporter {
        import_count: AtomicUsize::new(0),
    })];
    let options = CookOptions {
        manifest_path: fx.manifest_path,
        source_root: fx.source_root.clone(),
        output_root: fx.output_root.clone(),
    };

    // Hand-write an index that is byte-for-byte current for its source and
    // dependency, but stamped with an unknown cook-format version.
    let source_path = fx.source_root.join("thing.fake");
    let dep_path = fx.source_root.join("thing.dep");
    let stale = SourceIndex {
        format_version: 999,
        source_path: source_path.clone(),
        source_hash: asset_cook::hash_file_contents(&source_path).unwrap(),
        sub_assets: vec![],
        dependencies: vec![DependencyEntry {
            content_hash: asset_cook::hash_file_contents(&dep_path).unwrap(),
            path: dep_path,
        }],
    };
    let index_dir = fx.output_root.join(".index");
    std::fs::create_dir_all(&index_dir).unwrap();
    std::fs::write(
        index_dir.join("thing.fake.bin"),
        bincode::serialize(&stale).unwrap(),
    )
    .unwrap();

    let report = run_cook(&importers, &options);
    assert_eq!(
        report.errors.len(),
        0,
        "cook must succeed: {:?}",
        report.errors
    );
    assert_eq!(
        report.cooked.len(),
        1,
        "an index written by an unknown COOK_FORMAT_VERSION must be rebuilt, not skipped"
    );
    assert_eq!(
        report.skipped.len(),
        0,
        "the stale-format source must not be skipped"
    );
}
