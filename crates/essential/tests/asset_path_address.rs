//! Regression coverage for a bug found in review: `AssetServer::load_internal`
//! derives its `AssetId` from an `AssetPath`, but `AssetPath::new` always
//! prepends "res/" during normalization. If `load_internal` hashed the fully
//! normalized path directly, a runtime `load("models/character.gltf#scene")`
//! call would compute a *different* `AssetId` than cook-time code computing
//! `AssetId::from_path("models/character.gltf#scene")` from the raw,
//! manifest-relative address (no "res/" prefix) — silently breaking every
//! top-level `load()` call once cooked assets are looked up by ID.
//!
//! `AssetPath::address()` exists to recover the original, un-prefixed
//! logical address, so `load_internal` can hash the *same* string cook-time
//! code hashes. These tests pin that contract at the `AssetPath` level,
//! which is what `load_internal` now relies on.
use essential::assets::{AssetId, AssetPath};

#[test]
fn address_strips_the_res_prefix_new_adds() {
    let path = AssetPath::new("models/character.gltf#scene");

    // AssetPath::new always adds "res/" during normalization...
    assert!(path.to_path().to_string_lossy().starts_with("res/"));
    // ...but `address()` recovers the original, un-prefixed string.
    assert_eq!(path.address(), "models/character.gltf#scene");
}

#[test]
fn address_agrees_with_cook_time_id_computation() {
    // This is the actual invariant load_internal depends on: hashing
    // AssetPath::new(raw).address() must produce the same AssetId as
    // hashing `raw` directly, which is what cook-time code (assets.toml
    // entries, ImportContext::sub_asset_id, etc.) does with no AssetPath
    // involved at all.
    let raw_address = "models/character.gltf#scene";

    let runtime_id = AssetId::from_path(&AssetPath::new(raw_address).address());
    let cook_time_id = AssetId::from_path(raw_address);

    assert_eq!(
        runtime_id, cook_time_id,
        "a runtime load() call and cook-time ID computation must agree on the AssetId for the same asset address"
    );
}

#[test]
fn address_is_idempotent_when_res_prefix_already_present() {
    // Whether or not the caller happened to already type "res/" up front,
    // address() must recover the same logical address either way.
    let with_prefix = AssetPath::new("res/models/character.gltf#scene");
    let without_prefix = AssetPath::new("models/character.gltf#scene");

    assert_eq!(with_prefix.address(), without_prefix.address());
}
