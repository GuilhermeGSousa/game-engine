//! `import` turns a glTF into content assets whose cross-references address
//! content-tree paths, not `<source>#<sub>`.
use std::path::{Path, PathBuf};

use ecs::component::Component;
use essential::assets::content::{read_content_asset, AssetRegistry, ImportProvenance};
use essential::assets::{Asset, AssetId};
use mesh::mesh::MeshComponent;
use scene::scene::Scene;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../gltf-loader/tests/fixtures/triangle.gltf")
        .canonicalize()
        .expect("fixture exists")
}

#[test]
fn writes_content_assets_with_content_path_cross_references() {
    let project_root = std::env::temp_dir().join(format!("import-gltf-{}", std::process::id()));
    std::fs::create_dir_all(&project_root).unwrap();

    let written = import::import_source(&fixture(), &project_root, &Default::default())
        .expect("import succeeds");

    for expected in [
        "content/triangle/mesh_0.gasset",
        "content/triangle/scene.gasset",
    ] {
        assert!(
            written.iter().any(|a| a.address == expected),
            "expected {expected} among {written:?}"
        );
        assert!(project_root.join(expected).exists(), "{expected} on disk");
    }

    let raw = std::fs::read(project_root.join("content/triangle/scene.gasset")).unwrap();
    let (header, payload) = read_content_asset(&raw).expect("readable");
    assert_eq!(header.kind, Scene::name());
    assert_eq!(
        header.asset_id,
        AssetId::from_path("content/triangle/scene.gasset")
    );
    assert_eq!(
        header.provenance,
        Some(ImportProvenance {
            source: fixture().display().to_string(),
            sub_asset: "scene".to_string(),
        }),
        "import must record where the content asset came from"
    );

    let scene: Scene = bincode::deserialize(payload).expect("scene payload");
    let mesh_id = AssetId::from_path("content/triangle/mesh_0.gasset");
    let component = scene.nodes[0]
        .components
        .iter()
        .find(|c| c.type_name == MeshComponent::name())
        .expect("the node carries a MeshComponent");
    let referenced = serde_json::from_str::<MeshComponent>(&component.data)
        .expect("a MeshComponent payload must deserialize")
        .handle
        .id();
    assert_eq!(
        referenced, mesh_id,
        "the MeshComponent must address the content path, not triangle.gltf#mesh/0"
    );
    assert!(
        header.references.contains(&mesh_id),
        "header references list carries the content-path mesh id"
    );

    std::fs::remove_dir_all(&project_root).ok();
}

#[test]
fn import_upserts_the_registry_for_every_written_asset() {
    let project_root =
        std::env::temp_dir().join(format!("import-gltf-registry-{}", std::process::id()));
    std::fs::create_dir_all(&project_root).unwrap();

    let written = import::import_source(&fixture(), &project_root, &Default::default())
        .expect("import succeeds");

    let registry = AssetRegistry::load(&project_root).expect("registry loads");
    for asset in &written {
        assert_eq!(
            registry.get(AssetId::from_path(&asset.address)),
            Some(asset.address.as_str()),
            "import must register every content asset it writes"
        );
    }

    std::fs::remove_dir_all(&project_root).ok();
}
