//! `AssetPath::address()` recovers the exact address a caller passed to
//! `load()` (or that `import`'s content-address convention produced), so
//! `AssetId::from_path(path.address())` — what `AssetServer::load_internal`
//! computes — agrees with an `AssetId::from_path` computed directly from the
//! same string with no `AssetPath` involved (e.g. `import`'s
//! `ImportContext::sub_asset_id`, or a hand-written test id).
use essential::assets::{AssetId, AssetPath};

#[test]
fn address_returns_the_normalized_path_unchanged() {
    let path = AssetPath::new("content/hero/scene.gasset");
    assert_eq!(path.address(), "content/hero/scene.gasset");
}

#[test]
fn address_normalizes_backslashes_and_a_leading_dot_slash() {
    let path = AssetPath::new("./content\\hero\\scene.gasset");
    assert_eq!(path.address(), "content/hero/scene.gasset");
}

#[test]
fn address_agrees_with_import_time_id_computation() {
    let raw_address = "content/hero/scene.gasset";

    let runtime_id = AssetId::from_path(&AssetPath::new(raw_address).address());
    let import_time_id = AssetId::from_path(raw_address);

    assert_eq!(
        runtime_id, import_time_id,
        "a runtime load() call and import-time ID computation must agree on the AssetId for the same asset address"
    );
}
