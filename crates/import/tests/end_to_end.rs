//! The full Plan 1 loop: import writes content assets, the runtime byte
//! path reads them back as the right type, and an editor-saved Scene
//! round-trips.
//!
//! These drive `load_asset_bytes` — the exact helper every AssetLoader
//! calls — rather than `AssetServer::load`, because completing an
//! AssetServer load needs the LoadTaskPool plus a World to pump
//! `handle_asset_load_events`, and no such harness exists in the test
//! suite today. This covers the same code path minus the task-pool wrapper.
use std::path::{Path, PathBuf};

use essential::assets::content::{read_content_asset, save_content_asset};
use essential::assets::utils::load_asset_bytes;
use essential::assets::{Asset, AssetId, CookedAssetRoot};
use mesh::mesh::Mesh;
use scene::scene::Scene;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../gltf-loader/tests/fixtures/triangle.gltf")
        .canonicalize()
        .expect("fixture exists")
}

#[test]
fn imported_content_assets_load_back_as_their_type() {
    let root = std::env::temp_dir().join(format!("content-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    import::import_source(&fixture(), &root, &Default::default()).expect("import");

    let address = "content/triangle/mesh_0.gasset";
    let bytes = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(root.clone()),
        address,
        AssetId::from_path(address),
        Mesh::name(),
    ))
    .expect("the imported content asset loads");

    let mesh: Mesh = bincode::deserialize(&bytes).expect("payload is a Mesh");
    assert_eq!(
        mesh.vertices.len(),
        3,
        "the triangle fixture's mesh survives import -> load"
    );

    // The same file refuses to load as the wrong type.
    let err = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(root.clone()),
        address,
        AssetId::from_path(address),
        Scene::name(),
    ))
    .expect_err("kind tag must reject a Scene load of a Mesh file");
    assert!(format!("{err:#}").contains("Mesh"));

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn an_editor_saved_scene_round_trips() {
    let root = std::env::temp_dir().join(format!("content-save-e2e-{}", std::process::id()));
    let address = "content/levels/intro.gasset";

    let scene = Scene {
        nodes: Vec::new(),
        referenced_assets: Vec::new(),
    };
    save_content_asset(&scene, &root, address).expect("save");

    let bytes = pollster::block_on(load_asset_bytes(
        &CookedAssetRoot::Directory(root.clone()),
        address,
        AssetId::from_path(address),
        Scene::name(),
    ))
    .expect("an editor-saved asset loads through the runtime path");
    let restored: Scene = bincode::deserialize(&bytes).expect("payload is a Scene");
    assert_eq!(restored.nodes.len(), scene.nodes.len());

    // And its header is well-formed.
    let raw = std::fs::read(root.join(address)).unwrap();
    let (header, _) = read_content_asset(&raw).expect("readable");
    assert_eq!(header.kind, Scene::name());
    assert_eq!(header.asset_id, AssetId::from_path(address));

    std::fs::remove_dir_all(&root).ok();
}
