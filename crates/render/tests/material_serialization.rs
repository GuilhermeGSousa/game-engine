//! Covers StandardMaterial round-tripping directly through bincode (no DTO)
//! and reporting its texture references for cook-time validation.
use essential::assets::{handle::AssetHandle, AssetId};
use render::assets::material::StandardMaterial;
use render::assets::texture::Texture;

#[test]
fn round_trips_through_bincode_with_weak_texture_handles() {
    let albedo_id = AssetId::from_path("models/character.gltf#texture/albedo");
    let material = StandardMaterial::new(Some(AssetHandle::weak(albedo_id)), None);

    let bytes =
        bincode::serialize(&material).expect("StandardMaterial should serialize via bincode");
    let decoded: StandardMaterial = bincode::deserialize(&bytes)
        .expect("StandardMaterial should deserialize from bincode bytes");

    assert_eq!(
        decoded.base_color_texture().map(|h| h.id()),
        Some(albedo_id),
        "the deserialized material's texture field must carry the same AssetId, as a Weak handle"
    );
}

#[test]
fn referenced_sub_assets_lists_present_textures_only() {
    let albedo_id = AssetId::from_path("models/character.gltf#texture/albedo");
    let material = StandardMaterial::new(Some(AssetHandle::<Texture>::weak(albedo_id)), None);

    let refs = asset_cook::CookedAsset::referenced_sub_assets(&material);
    assert_eq!(
        refs,
        vec![albedo_id],
        "only Some(..) texture fields should be reported as references"
    );
}
