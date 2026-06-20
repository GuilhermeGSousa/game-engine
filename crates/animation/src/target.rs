use ecs::{
    component::Component,
    entity::Entity,
    query::{
        Query,
        query_filter::{Added, Changed},
    },
    resource::Res,
};
use essential::{assets::asset_store::AssetStore, time::Time, transform::Transform};
use uuid::Uuid;

use crate::{
    clip::AnimationClip,
    evaluation::AnimationGraphContext,
    graph::AnimationGraph,
    player::{AnimationHandleComponent, AnimationPlayer},
    pose::AnimationSkeleton,
};

#[derive(Component)]
pub struct AnimationTarget {
    pub id: Uuid,
    pub animator: Entity,
}

/// Consumes the loader-provided [`AnimationSkeleton`] into the player's pose layout, once,
/// when the skeleton is first attached.
pub(crate) fn build_pose_layouts(
    animation_players: Query<&mut AnimationPlayer>,
    skeletons: Query<(Entity, &AnimationSkeleton), Added<AnimationSkeleton>>,
) {
    for (entity, skeleton) in skeletons.iter() {
        let Some(mut animation_player) = animation_players.get_entity(entity) else {
            continue;
        };

        animation_player.set_layout(skeleton.bones.clone());
    }
}

/// Evaluates each player's graph exactly once per frame into its full-skeleton pose.
pub(crate) fn evaluate_animations(
    animation_players: Query<&mut AnimationPlayer>,
    animation_clips: Res<AssetStore<AnimationClip>>,
    animation_graphs: Res<AssetStore<AnimationGraph>>,
) {
    for mut animation_player in animation_players.iter() {
        animation_player.evaluate(&AnimationGraphContext {
            animation_clips: &animation_clips,
            animation_graphs: &animation_graphs,
        });
    }
}

/// Writes each player's evaluated pose back onto its individual bone transforms.
pub(crate) fn apply_poses(
    animation_players: Query<&AnimationPlayer>,
    transforms: Query<&mut Transform>,
) {
    for animation_player in animation_players.iter() {
        let pose = animation_player.current_pose();

        for (index, bone) in animation_player.layout().bones().iter().enumerate() {
            if bone.is_root {
                // TODO: Accumulate root motion; leave the root bone's transform untouched.
                continue;
            }

            if let Some(mut transform) = transforms.get_entity(bone.entity) {
                **transform = pose.transforms[index].clone();
            }
        }
    }
}

pub(crate) fn update_animation_players(
    animation_players: Query<&mut AnimationPlayer>,
    animation_clips: Res<AssetStore<AnimationClip>>,
    animation_graphs: Res<AssetStore<AnimationGraph>>,
    time: Res<Time>,
) {
    let delta_time = time.delta().as_secs_f32();
    for mut animation_player in animation_players.iter() {
        animation_player.update(
            delta_time,
            &AnimationGraphContext {
                animation_clips: &animation_clips,
                animation_graphs: &animation_graphs,
            },
        );
    }
}

pub(crate) fn initialize_animation_players(
    animation_players: Query<
        (&mut AnimationPlayer, &AnimationHandleComponent),
        Changed<AnimationHandleComponent>,
    >,
    animation_graphs: Res<AssetStore<AnimationGraph>>,
    animation_clips: Res<AssetStore<AnimationClip>>,
) {
    for (mut animation_player, graph_handle) in animation_players.iter() {
        animation_player.initialize_graph(
            (*graph_handle).clone(),
            &AnimationGraphContext {
                animation_clips: &animation_clips,
                animation_graphs: &animation_graphs,
            },
        );
    }
}
