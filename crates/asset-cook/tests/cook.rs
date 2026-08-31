//! Covers cooking a single source file end-to-end: an Importer emits
//! sub-assets, cook_source writes each to its flat, AssetId-keyed location.
use std::path::Path;

use asset_cook::{
    cook_source, cooked_file_path_for_id, CookedAsset, ImportContext, ImportError, Importer,
};
use essential::assets::AssetId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct FakeCookedThing {
    value: u32,
}

impl CookedAsset for FakeCookedThing {
    const TYPE_NAME: &'static str = "FakeThing";
}

struct FakeImporter;

impl Importer for FakeImporter {
    fn supported_extensions(&self) -> &'static [&'static str] {
        &["fake"]
    }

    fn import(&self, _source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError> {
        ctx.emit("thing/0", &FakeCookedThing { value: 7 }).unwrap();
        ctx.emit("thing/1", &FakeCookedThing { value: 9 }).unwrap();
        Ok(())
    }
}

#[test]
fn cook_source_writes_one_flat_file_per_sub_asset() {
    let temp_dir = std::env::temp_dir().join(format!("asset-cook-test-{}", std::process::id()));
    let source_root = temp_dir.join("assets");
    let output_root = temp_dir.join("res");
    std::fs::create_dir_all(source_root.join("models")).unwrap();
    let relative_source = Path::new("models/character.fake");
    let source_path = source_root.join(relative_source);
    std::fs::write(&source_path, b"fake source content").unwrap();

    let index = cook_source(&FakeImporter, &source_path, relative_source, &output_root)
        .expect("cooking a valid fake source should succeed");

    assert_eq!(
        index.sub_assets.len(),
        2,
        "both emitted sub-assets must appear in the index"
    );

    let expected_id = AssetId::from_path("models/character.fake#thing/0");
    assert_eq!(index.sub_assets[0].asset_id, expected_id);

    let cooked_path = cooked_file_path_for_id(&output_root, expected_id);
    assert!(
        cooked_path.exists(),
        "cooked file must exist at the deterministic ID-keyed path"
    );

    let stem = cooked_path.file_stem().unwrap().to_str().unwrap();
    assert_eq!(
        stem.len(),
        32,
        "cooked filename stem must be the 32-char hyphenless UUID hex"
    );
    assert!(
        stem.chars().all(|c| c.is_ascii_hexdigit()),
        "cooked filename stem must be pure hex: {stem}"
    );

    let bytes = std::fs::read(&cooked_path).unwrap();
    let decoded: FakeCookedThing = bincode::deserialize(&bytes).unwrap();
    assert_eq!(decoded.value, 7);

    std::fs::remove_dir_all(&temp_dir).ok();
}
