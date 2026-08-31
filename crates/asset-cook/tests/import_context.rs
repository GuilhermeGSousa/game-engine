//! Covers ImportContext's sub-asset emission, same-file reference-ID
//! computation, and dependency tracking.
use asset_cook::{CookedAsset, ImportContext};
use essential::assets::AssetId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct FakeCookedThing {
    referenced: AssetId,
}

impl CookedAsset for FakeCookedThing {
    const TYPE_NAME: &'static str = "FakeThing";

    fn referenced_sub_assets(&self) -> Vec<AssetId> {
        vec![self.referenced]
    }
}

#[test]
fn sub_asset_id_is_stable_and_scoped_to_the_source_file() {
    let ctx = ImportContext::new(std::path::PathBuf::from("models/character.gltf"));
    let id_a = ctx.sub_asset_id("mesh/0");
    let id_b = ctx.sub_asset_id("mesh/0");
    assert_eq!(id_a, id_b, "the same name in the same context must always resolve to the same id");
    assert_eq!(
        id_a,
        AssetId::from_path("models/character.gltf#mesh/0"),
        "sub_asset_id must match what a runtime load of the fully-qualified path would compute"
    );
}

#[test]
fn emit_records_sub_asset_with_serialized_bytes_and_references() {
    let mut ctx = ImportContext::new(std::path::PathBuf::from("models/character.gltf"));
    let referenced_id = ctx.sub_asset_id("texture/albedo");
    let thing = FakeCookedThing { referenced: referenced_id };

    ctx.emit("material/0", &thing).expect("emit should succeed for a serializable value");

    let (sub_assets, _dependencies) = ctx.into_parts();
    assert_eq!(sub_assets.len(), 1);

    let entry = &sub_assets[0];
    assert_eq!(entry.name, "material/0");
    assert_eq!(entry.asset_id, AssetId::from_path("models/character.gltf#material/0"));
    assert_eq!(entry.type_name, "FakeThing");
    assert_eq!(entry.references, vec![referenced_id]);

    let round_tripped: FakeCookedThing =
        bincode::deserialize(&entry.bytes).expect("emitted bytes must deserialize back");
    assert_eq!(round_tripped.referenced, referenced_id);
}

#[test]
fn track_dependency_records_path_and_hash() {
    let mut ctx = ImportContext::new(std::path::PathBuf::from("models/character.gltf"));
    ctx.track_dependency(std::path::PathBuf::from("assets/models/character.bin"), 12345);

    let (_sub_assets, dependencies) = ctx.into_parts();
    assert_eq!(dependencies.len(), 1);
    assert_eq!(dependencies[0].path, std::path::PathBuf::from("assets/models/character.bin"));
    assert_eq!(dependencies[0].content_hash, 12345);
}
