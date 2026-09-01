//! Covers GltfImporter splitting a single .gltf into independently-cooked
//! mesh, material, and scene sub-assets (this fixture has no textures),
//! with the scene's node referencing the mesh/material by stable AssetId.
use std::path::Path;

use asset_cook::{ImportContext, Importer};
use essential::assets::AssetId;
use gltf_loader::gltf_importer::GltfImporter;
use scene::scene::Scene;

#[test]
fn import_emits_mesh_material_and_scene_sub_assets() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/triangle.gltf");
    let relative_source = Path::new("triangle.gltf");
    let mut ctx = ImportContext::new(relative_source.to_path_buf());

    GltfImporter
        .import(&fixture, &mut ctx)
        .expect("importing the triangle fixture should succeed");
    let outputs = ctx.into_parts();

    let names: Vec<&str> = outputs.sub_assets.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"mesh/0"),
        "expected a mesh/0 sub-asset, got: {names:?}"
    );
    assert!(
        names.contains(&"material/0"),
        "expected a material/0 sub-asset, got: {names:?}"
    );
    assert!(
        names.contains(&"scene"),
        "expected a scene sub-asset, got: {names:?}"
    );

    let scene_entry = outputs
        .sub_assets
        .iter()
        .find(|s| s.name == "scene")
        .unwrap();
    let cooked_scene: Scene = bincode::deserialize(&scene_entry.bytes).unwrap();
    assert_eq!(cooked_scene.nodes.len(), 1);
    assert_eq!(cooked_scene.nodes[0].name, "Triangle");
    assert_eq!(
        cooked_scene.nodes[0].mesh.as_ref().unwrap().id(),
        AssetId::from_path("triangle.gltf#mesh/0"),
        "the scene node's mesh handle must carry the exact same AssetId a runtime load of 'triangle.gltf#mesh/0' would compute"
    );
}
