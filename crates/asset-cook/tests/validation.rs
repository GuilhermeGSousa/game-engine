//! Covers cross-source reference-integrity checking and the per-Importer
//! validate() hook, both of which must fail a cook run when triggered.
use std::path::Path;

use asset_cook::{
    run_cook, CookOptions, CookedAsset, EmittedSubAsset, ImportContext, ImportError, Importer,
    ValidationIssue, ValidationSeverity,
};
use essential::assets::AssetId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct RefThing {
    references: Vec<AssetId>,
}

impl CookedAsset for RefThing {
    const TYPE_NAME: &'static str = "RefThing";
    fn referenced_sub_assets(&self) -> Vec<AssetId> {
        self.references.clone()
    }
}

struct DanglingRefImporter;

impl Importer for DanglingRefImporter {
    fn supported_extensions(&self) -> &'static [&'static str] {
        &["fake"]
    }
    fn import(&self, _source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError> {
        let dangling = AssetId::from_path("models/does_not_exist.fake#thing/0");
        ctx.emit("thing/0", &RefThing { references: vec![dangling] }).unwrap();
        Ok(())
    }
}

struct AlwaysErrorsImporter;

impl Importer for AlwaysErrorsImporter {
    fn supported_extensions(&self) -> &'static [&'static str] {
        &["fake"]
    }
    fn import(&self, _source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError> {
        ctx.emit("thing/0", &RefThing { references: vec![] }).unwrap();
        Ok(())
    }
    fn validate(&self, _sub_assets: &[EmittedSubAsset]) -> Vec<ValidationIssue> {
        vec![ValidationIssue {
            severity: ValidationSeverity::Error,
            message: "always fails for this test".to_string(),
            source_path: std::path::PathBuf::from("thing.fake"),
            sub_asset_name: Some("thing/0".to_string()),
        }]
    }
}

fn write_fixture(name: &str) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let temp_dir = std::env::temp_dir().join(format!("asset-cook-validation-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let source_root = temp_dir.join("assets");
    let output_root = temp_dir.join("res");
    std::fs::create_dir_all(&source_root).unwrap();
    std::fs::write(source_root.join("thing.fake"), b"source").unwrap();
    let manifest_path = temp_dir.join("assets.toml");
    std::fs::write(&manifest_path, "[[assets]]\npath = \"thing.fake\"\n").unwrap();
    (manifest_path, source_root, output_root)
}

#[test]
fn dangling_reference_fails_the_cook_run() {
    let (manifest_path, source_root, output_root) = write_fixture("dangling");
    let importers: Vec<Box<dyn Importer>> = vec![Box::new(DanglingRefImporter)];
    let options = CookOptions { manifest_path, source_root: source_root.clone(), output_root };

    let report = run_cook(&importers, &options);
    assert!(!report.errors.is_empty(), "a reference to a sub-asset that was never produced must fail the run");

    std::fs::remove_dir_all(source_root.parent().unwrap()).ok();
}

#[test]
fn validate_error_severity_fails_the_cook_run() {
    let (manifest_path, source_root, output_root) = write_fixture("validate-error");
    let importers: Vec<Box<dyn Importer>> = vec![Box::new(AlwaysErrorsImporter)];
    let options = CookOptions { manifest_path, source_root: source_root.clone(), output_root };

    let report = run_cook(&importers, &options);
    assert!(!report.errors.is_empty(), "an Error-severity ValidationIssue must fail the run");

    std::fs::remove_dir_all(source_root.parent().unwrap()).ok();
}
