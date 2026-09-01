//! Covers Scene/SceneNode round-tripping directly through bincode and
//! reporting mesh/material references for cook-time validation.
use asset_cook::CookedAsset;
use essential::assets::{handle::AssetHandle, AssetId};
use essential::transform::Transform;
use scene::scene::{Scene, SceneNode};

#[test]
fn round_trips_and_reports_references() {
    let mesh_id = AssetId::from_path("models/character.gltf#mesh/0");
    let material_id = AssetId::from_path("models/character.gltf#material/0");

    let sc = Scene {
        nodes: vec![
            SceneNode {
                name: "root".to_string(),
                transform: Transform::default(),
                children: vec![1],
                mesh: None,
                material: None,
            },
            SceneNode {
                name: "child".to_string(),
                transform: Transform::default(),
                children: vec![],
                mesh: Some(AssetHandle::weak(mesh_id)),
                material: Some(AssetHandle::weak(material_id)),
            },
        ],
    };

    let bytes = bincode::serialize(&sc).expect("Scene must serialize through bincode");
    let decoded: Scene =
        bincode::deserialize(&bytes).expect("Scene must deserialize through bincode");

    assert_eq!(
        decoded.nodes.len(),
        2,
        "both nodes must survive the round-trip"
    );
    assert_eq!(
        decoded.nodes[0].children,
        vec![1],
        "child indices are plain data and must round-trip verbatim"
    );
    assert_eq!(
        decoded.nodes[1].mesh.as_ref().unwrap().id(),
        mesh_id,
        "a node's mesh handle must deserialize back to the same AssetId"
    );

    assert_eq!(
        sc.referenced_sub_assets(),
        vec![mesh_id, material_id],
        "every node's mesh/material reference must be collected across the whole scene"
    );
}
