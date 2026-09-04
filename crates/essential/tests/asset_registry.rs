//! `AssetRegistry` load/save/get/insert/remove/iter, and `save_content_asset`
//! upserting it.
use essential::assets::content::{read_content_asset, save_content_asset, AssetRegistry};
use essential::assets::{Asset, AssetId};
use serde::{Deserialize, Serialize};

fn temp_root(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("asset-registry-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn insert_get_remove_round_trip_in_memory() {
    let mut registry = AssetRegistry::new();
    let id = AssetId::from_path("content/hero/scene.gasset");
    assert_eq!(registry.get(id), None);

    registry.insert(id, "content/hero/scene.gasset");
    assert_eq!(registry.get(id), Some("content/hero/scene.gasset"));

    assert_eq!(
        registry.remove(id),
        Some("content/hero/scene.gasset".to_string())
    );
    assert_eq!(registry.get(id), None);
}

#[test]
fn save_then_load_round_trips_every_entry() {
    let dir = temp_root("save-load");
    let mut registry = AssetRegistry::new();
    let a = AssetId::from_path("content/a.gasset");
    let b = AssetId::from_path("content/b.gasset");
    registry.insert(a, "content/a.gasset");
    registry.insert(b, "content/b.gasset");
    registry.save(&dir).expect("save");

    let loaded = AssetRegistry::load(&dir).expect("load");
    assert_eq!(loaded.get(a), Some("content/a.gasset"));
    assert_eq!(loaded.get(b), Some("content/b.gasset"));
    assert_eq!(loaded.iter().count(), 2);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn loading_a_registry_that_was_never_saved_is_empty_not_an_error() {
    let dir = temp_root("never-saved");
    let registry = AssetRegistry::load(&dir).expect("an absent registry file is not an error");
    assert_eq!(registry.iter().count(), 0);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn registry_file_lands_under_content_and_is_keyed_by_simple_hex() {
    let dir = temp_root("on-disk-shape");
    let mut registry = AssetRegistry::new();
    let id = AssetId::from_path("content/hero/scene.gasset");
    registry.insert(id, "content/hero/scene.gasset");
    registry.save(&dir).expect("save");

    let text = std::fs::read_to_string(dir.join("content/.registry.toml")).expect("file exists");
    assert!(
        text.contains(&id.simple_hex()),
        "the registry must key entries by simple_hex(), got: {text}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[derive(Debug, Serialize, Deserialize)]
struct Thing;

impl Asset for Thing {
    fn name() -> &'static str {
        "Thing"
    }
}

#[test]
fn save_content_asset_upserts_the_registry() {
    let dir = temp_root("save-upserts");
    let address = "content/things/one.gasset";

    save_content_asset(&Thing, &dir, address).expect("save");

    let registry = AssetRegistry::load(&dir).expect("load");
    assert_eq!(
        registry.get(AssetId::from_path(address)),
        Some(address),
        "save_content_asset must register the asset it just wrote"
    );

    let raw = std::fs::read(dir.join(address)).unwrap();
    let (header, _) = read_content_asset(&raw).expect("readable");
    assert_eq!(
        header.provenance, None,
        "an editor save is not import-derived"
    );

    std::fs::remove_dir_all(&dir).ok();
}
