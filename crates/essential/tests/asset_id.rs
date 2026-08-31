//! Covers AssetId::from_path's determinism (same input -> same ID, every
//! run) and its bincode round-trip, both load-bearing for the cook pipeline:
//! cook-time and run-time must independently compute the same ID from the
//! same "path#fragment" string with no shared state.
use essential::assets::AssetId;

#[test]
fn from_path_is_deterministic() {
    let a = AssetId::from_path("models/character.gltf#texture/albedo");
    let b = AssetId::from_path("models/character.gltf#texture/albedo");
    assert_eq!(
        a, b,
        "the same path string must hash to the same AssetId every time"
    );
}

#[test]
fn from_path_differs_for_different_inputs() {
    let a = AssetId::from_path("models/character.gltf#texture/albedo");
    let b = AssetId::from_path("models/character.gltf#texture/normal");
    assert_ne!(a, b, "distinct sub-asset names must hash to distinct IDs");
}

#[test]
fn round_trips_through_bincode() {
    let id = AssetId::from_path("models/character.gltf#scene");
    let bytes = bincode::serialize(&id).unwrap();
    let decoded: AssetId = bincode::deserialize(&bytes).unwrap();
    assert_eq!(decoded, id);
}
