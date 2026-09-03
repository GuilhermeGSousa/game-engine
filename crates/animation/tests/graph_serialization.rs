use animation::clip::AnimationClip;
use animation::graph::AnimationGraph;
use animation::node::AnimationNodeKind;
use essential::assets::AssetId;
use essential::assets::handle::AssetHandle;

#[test]
fn animation_graph_round_trips_through_bincode() {
    let clip = |name: &str| AssetHandle::<AnimationClip>::weak(AssetId::from_path(name));

    let mut graph = AnimationGraph::new();
    graph
        .result_node()
        .with_blend_space_2d_input("movement", |ctx| {
            ctx.animation_clip_input(clip("idle"), glam::Vec2::ZERO)
                .animation_clip_input(clip("walk"), glam::Vec2::new(0.0, 1.0))
                .animation_clip_input(clip("strafe"), glam::Vec2::new(1.0, 0.0));
        });

    let bytes = bincode::serialize(&graph).expect("serialize");
    let restored: AnimationGraph = bincode::deserialize(&bytes).expect("deserialize");

    let count = |g: &AnimationGraph| g.iter().count();
    assert_eq!(count(&restored), count(&graph), "node count survives");

    // Blackboard param on the blend-space node survives.
    let has_movement_input = restored.iter().any(|idx| {
        matches!(
            restored.get_node(idx),
            Some(AnimationNodeKind::BlendSpace2D(def)) if def.input().param == "movement"
        )
    });
    assert!(
        has_movement_input,
        "BlendSpace2D input param survived the round trip"
    );
}
