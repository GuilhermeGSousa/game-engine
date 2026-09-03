//! Covers the expansion case of the SceneComponent interface: SceneSkeleton
//! is authoring data that becomes several runtime components -- one of them
//! on a *different* entity -- and never lands on an entity itself.
use animation::player::AnimationPlayer;
use animation::root::AnimationRootBone;
use ecs::component::scene::{SceneComponent, SceneEntityRef, SceneSpawnContext};
use ecs::World;
use essential::assets::{handle::AssetHandle, AssetId};
use mesh::skeleton::{Skeleton, SkeletonComponent};
use scene::skeleton::{SceneSkeleton, SceneSkeletonBinding};
use uuid::Uuid;

#[test]
fn scene_skeleton_expands_into_runtime_components() {
    let mut world = World::default();
    let owner = world.spawn(());
    let bone_a = world.spawn(());
    let bone_b = world.spawn(());
    let nodes = [owner, bone_a, bone_b];

    let authoring = SceneSkeleton {
        skeleton: AssetHandle::<Skeleton>::weak(AssetId::from_path("rig.gltf#skeleton/0")),
        bones: vec![SceneEntityRef(1), SceneEntityRef(2)],
        bone_ids: vec![Uuid::from_u128(1), Uuid::from_u128(2)],
        root: Some(SceneEntityRef(1)),
    };

    {
        let mut ctx = SceneSpawnContext::new((&mut world).into(), &nodes);
        authoring.apply(owner, &mut ctx);
    }

    let skeleton_component = world
        .get_component_for_entity::<SkeletonComponent>(owner)
        .expect("apply must insert a SkeletonComponent on its own entity");
    assert_eq!(
        skeleton_component.bones(),
        &[bone_a, bone_b],
        "SceneEntityRef indices must resolve to the spawned bone entities"
    );

    assert!(
        world
            .get_component_for_entity::<AnimationPlayer>(owner)
            .is_some(),
        "apply must insert an AnimationPlayer sized to the bone count"
    );
    assert!(
        world
            .get_component_for_entity::<AnimationRootBone>(bone_a)
            .is_some(),
        "the root bone marker must land on the root bone's entity, not the owner"
    );
    assert!(
        world
            .get_component_for_entity::<SceneSkeleton>(owner)
            .is_none(),
        "authoring data must never insert one of itself"
    );
}

#[test]
fn scene_skeleton_binding_skins_without_a_player() {
    let mut world = World::default();
    let owner = world.spawn(());
    let bone_a = world.spawn(());
    let bone_b = world.spawn(());
    let nodes = [owner, bone_a, bone_b];

    let binding = SceneSkeletonBinding {
        skeleton: AssetHandle::<Skeleton>::weak(AssetId::from_path("rig.gltf#skeleton/0")),
        bones: vec![SceneEntityRef(1), SceneEntityRef(2)],
        bone_ids: vec![Uuid::from_u128(1), Uuid::from_u128(2)],
    };

    {
        let mut ctx = SceneSpawnContext::new((&mut world).into(), &nodes);
        binding.apply(owner, &mut ctx);
    }

    let skeleton_component = world
        .get_component_for_entity::<SkeletonComponent>(owner)
        .expect("the binding must insert a SkeletonComponent on its own entity");
    assert_eq!(
        skeleton_component.bones(),
        &[bone_a, bone_b],
        "SceneEntityRef indices must resolve to the spawned bone entities"
    );
    assert!(
        world
            .get_component_for_entity::<AnimationPlayer>(owner)
            .is_none(),
        "the binding shares the owning node's player and must not add its own"
    );
    assert!(
        world
            .get_component_for_entity::<SceneSkeletonBinding>(owner)
            .is_none(),
        "authoring data must never insert one of itself"
    );
}

#[test]
fn out_of_range_bone_refs_are_skipped() {
    let mut world = World::default();
    let owner = world.spawn(());
    let nodes = [owner];

    let authoring = SceneSkeleton {
        skeleton: AssetHandle::<Skeleton>::weak(AssetId::from_path("rig.gltf#skeleton/0")),
        bones: vec![SceneEntityRef(9)],
        bone_ids: vec![Uuid::from_u128(1)],
        root: Some(SceneEntityRef(9)),
    };

    {
        let mut ctx = SceneSpawnContext::new((&mut world).into(), &nodes);
        authoring.apply(owner, &mut ctx);
    }

    let skeleton_component = world
        .get_component_for_entity::<SkeletonComponent>(owner)
        .expect("a malformed scene must still produce a component, not a panic");
    assert!(
        skeleton_component.bones().is_empty(),
        "an out-of-range bone reference must be dropped rather than panicking"
    );
}
