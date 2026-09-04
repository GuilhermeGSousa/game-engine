//! load_asset_bytes prefers a content asset at <root>/<address> and falls
//! back to the cooked <root>/.cooked/<hex>.bin layout.
use essential::assets::content::{write_content_asset, ContentAssetHeader, CONTENT_FORMAT_VERSION};
use essential::assets::utils::load_asset_bytes;
use essential::assets::{AssetId, CookedAssetRoot};

fn temp_root(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("content-first-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn prefers_a_content_asset_over_the_cooked_file() {
    let dir = temp_root("prefers");
    let address = "content/x/mesh_0.gasset";
    let id = AssetId::from_path(address);

    // Cooked file with one payload...
    std::fs::create_dir_all(dir.join(".cooked")).unwrap();
    std::fs::write(
        dir.join(format!(".cooked/{}.bin", id.simple_hex())),
        b"cooked",
    )
    .unwrap();
    // ...and a content asset with another, at the literal address.
    let header = ContentAssetHeader {
        format_version: CONTENT_FORMAT_VERSION,
        asset_id: id,
        references: Vec::new(),
        kind: "Mesh".to_string(),
    };
    std::fs::create_dir_all(dir.join("content/x")).unwrap();
    std::fs::write(
        dir.join(address),
        write_content_asset(&header, b"content").unwrap(),
    )
    .unwrap();

    let bytes = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(dir.clone()),
        address,
        id,
        "Mesh",
    ))
    .expect("load");
    assert_eq!(
        bytes, b"content",
        "the content asset wins and is header-stripped"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn falls_back_to_the_cooked_layout_when_no_content_asset_exists() {
    let dir = temp_root("fallback");
    let address = "Sponza/Sponza.gltf#scene";
    let id = AssetId::from_path(address);

    std::fs::create_dir_all(dir.join(".cooked")).unwrap();
    std::fs::write(
        dir.join(format!(".cooked/{}.bin", id.simple_hex())),
        b"cooked",
    )
    .unwrap();

    let bytes = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(dir.clone()),
        address,
        id,
        "Scene",
    ))
    .expect("load");
    assert_eq!(bytes, b"cooked");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_kind_mismatch_is_an_error() {
    let dir = temp_root("kind");
    let address = "content/x/thing.gasset";
    let id = AssetId::from_path(address);
    let header = ContentAssetHeader {
        format_version: CONTENT_FORMAT_VERSION,
        asset_id: id,
        references: Vec::new(),
        kind: "Mesh".to_string(),
    };
    std::fs::create_dir_all(dir.join("content/x")).unwrap();
    std::fs::write(
        dir.join(address),
        write_content_asset(&header, b"payload").unwrap(),
    )
    .unwrap();

    let err = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(dir.clone()),
        address,
        id,
        "Scene",
    ))
    .expect_err("kind mismatch must fail, not fall through to the cooked path");
    let message = format!("{err:#}");
    assert!(
        message.contains("Mesh") && message.contains("Scene"),
        "got: {message}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_hash_fragment_address_never_probes_for_a_content_asset() {
    let dir = temp_root("fragment");
    // Cook-style "<source>#<sub>" addresses only exist in the manifest-cook
    // world; a "#" would corrupt the path join / request URL, so they must
    // go straight to the cooked layout with no content probe.
    let address = "model.glb#mesh_0";
    let id = AssetId::from_path(address);

    std::fs::create_dir_all(dir.join(".cooked")).unwrap();
    std::fs::write(
        dir.join(format!(".cooked/{}.bin", id.simple_hex())),
        b"cooked",
    )
    .unwrap();
    // A valid content asset sitting at the literal joined path would win if it
    // were ever read — asserting the cooked bytes come back proves it is not.
    let header = ContentAssetHeader {
        format_version: CONTENT_FORMAT_VERSION,
        asset_id: id,
        references: Vec::new(),
        kind: "Mesh".to_string(),
    };
    std::fs::write(
        dir.join(address),
        write_content_asset(&header, b"content").unwrap(),
    )
    .unwrap();

    let bytes = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(dir.clone()),
        address,
        id,
        "Mesh",
    ))
    .expect("load");
    assert_eq!(
        bytes, b"cooked",
        "a '#' address must not read the content file beside it"
    );

    std::fs::remove_dir_all(&dir).ok();
}
