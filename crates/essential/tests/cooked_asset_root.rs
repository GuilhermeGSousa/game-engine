//! Covers the runtime cooked-byte loader against a Directory root. The
//! `.cooked/<simple_hex>.bin` layout here must match what
//! `asset_cook::cooked_file_path_for_id` writes at cook time — if these two
//! ever disagree, every cooked asset fails to load at runtime.
use essential::assets::utils::load_cooked_asset_bytes;
use essential::assets::{AssetId, CookedAssetRoot};

#[test]
fn reads_cooked_bytes_from_a_directory_root() {
    let temp_dir = std::env::temp_dir().join(format!("cooked-root-{}-read", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join(".cooked")).unwrap();

    let id = AssetId::from_path("models/character.gltf#mesh/0");
    std::fs::write(
        temp_dir
            .join(".cooked")
            .join(format!("{}.bin", id.simple_hex())),
        b"cooked-payload",
    )
    .unwrap();

    let root = CookedAssetRoot::Directory(temp_dir.clone());
    let bytes = pollster::block_on(load_cooked_asset_bytes(&root, id))
        .expect("a cooked file written at the ID-keyed path must be readable");

    assert_eq!(
        bytes, b"cooked-payload",
        "loader must return the cooked file's exact bytes"
    );

    std::fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn missing_cooked_file_errors_with_the_resolved_path() {
    let temp_dir = std::env::temp_dir().join(format!("cooked-root-{}-missing", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let id = AssetId::from_path("models/absent.gltf#mesh/0");
    let root = CookedAssetRoot::Directory(temp_dir.clone());
    let err = pollster::block_on(load_cooked_asset_bytes(&root, id))
        .expect_err("a missing cooked file must be an error, not empty bytes");

    let message = format!("{err:#}");
    assert!(
        message.contains(&id.simple_hex()),
        "the error must name the resolved cooked path so a missing asset is traceable; got: {message}"
    );

    std::fs::remove_dir_all(&temp_dir).ok();
}
