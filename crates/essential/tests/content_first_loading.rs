//! load_asset_bytes prefers a content asset at <root>/<address> and falls
//! back to the cooked <root>/.cooked/<hex>.bin layout.
use essential::assets::content::{write_content_asset, ContentAssetHeader, CONTENT_FORMAT_VERSION};
use essential::assets::utils::load_asset_bytes;
use essential::assets::{AssetId, CookedAssetRoot};
use serde::{Deserialize, Serialize};

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
        provenance: None,
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
        provenance: None,
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
        provenance: None,
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

#[test]
fn an_empty_address_never_probes_and_falls_back_to_cooked() {
    let dir = temp_root("empty-address");
    // Every path-less load (`AssetServer::load_by_id`) reaches this helper with
    // address "". Joined onto the root that names the root directory itself,
    // whose read is not `NotFound` — the empty address must short-circuit
    // straight to the cooked layout, never probe.
    let id = AssetId::from_path("content/x/mesh_0.gasset");

    std::fs::create_dir_all(dir.join(".cooked")).unwrap();
    std::fs::write(
        dir.join(format!(".cooked/{}.bin", id.simple_hex())),
        b"cooked",
    )
    .unwrap();

    let bytes = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(dir.clone()),
        "",
        id,
        "Mesh",
    ))
    .expect("an empty address falls back to the cooked layout, not an IsADirectory error");
    assert_eq!(bytes, b"cooked");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_address_that_names_a_directory_falls_back_instead_of_erroring() {
    let dir = temp_root("addr-is-dir");
    // `try_read_relative` must treat "the join is a directory, not a file" as
    // absence so the cooked fallback still runs.
    let address = "content/x";
    let id = AssetId::from_path(address);

    std::fs::create_dir_all(dir.join(address)).unwrap();
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
        "Mesh",
    ))
    .expect("a directory at the joined path is absence, not a fatal error");
    assert_eq!(bytes, b"cooked");

    std::fs::remove_dir_all(&dir).ok();
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct StandInMesh {
    vertices: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

#[test]
fn a_real_loader_reads_a_content_asset_and_falls_back() {
    // `MeshLoader::load` is exactly `load_asset_bytes(.., "Mesh")` then
    // `bincode::deserialize`. `essential` cannot depend on `mesh` (that is a
    // dependency cycle) and `AssetLoadContext::new` is crate-private, so this
    // exercises the identical bytes-then-decode path with a stand-in payload.
    let dir = temp_root("real-loader");
    let address = "content/x/m.gasset";
    let id = AssetId::from_path(address);
    let mesh = StandInMesh {
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        indices: vec![0, 1, 2],
    };

    // (a) a content asset at the literal address decodes back to the payload.
    let header = ContentAssetHeader {
        format_version: CONTENT_FORMAT_VERSION,
        asset_id: id,
        references: Vec::new(),
        kind: "Mesh".to_string(),
        provenance: None,
    };
    std::fs::create_dir_all(dir.join("content/x")).unwrap();
    let payload = bincode::serialize(&mesh).unwrap();
    std::fs::write(
        dir.join(address),
        write_content_asset(&header, &payload).unwrap(),
    )
    .unwrap();

    let bytes = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(dir.clone()),
        address,
        id,
        "Mesh",
    ))
    .expect("the content asset loads through the loader byte path");
    let decoded: StandInMesh = bincode::deserialize(&bytes).expect("payload is the mesh type");
    assert_eq!(
        decoded, mesh,
        "the loader path returns a decodable Mesh payload"
    );

    // (b) with the content file gone, the same call reads the cooked mirror.
    std::fs::remove_file(dir.join(address)).unwrap();
    std::fs::create_dir_all(dir.join(".cooked")).unwrap();
    let cooked_payload = bincode::serialize(&StandInMesh {
        vertices: vec![[9.0, 9.0, 9.0]],
        indices: vec![0],
    })
    .unwrap();
    std::fs::write(
        dir.join(format!(".cooked/{}.bin", id.simple_hex())),
        &cooked_payload,
    )
    .unwrap();

    let bytes = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(dir.clone()),
        address,
        id,
        "Mesh",
    ))
    .expect("with no content asset the loader byte path reads the cooked mirror");
    assert_eq!(
        bytes, cooked_payload,
        "the cooked payload comes back verbatim"
    );

    std::fs::remove_dir_all(&dir).ok();
}
