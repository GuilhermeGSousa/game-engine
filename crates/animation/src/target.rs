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
use mesh::skeleton::SkeletonComponent;
use uuid::Uuid;

use crate::{
    clip::AnimationClip,
    evaluation::AnimationGraphContext,
    graph::AnimationGraph,
    player::{AnimationHandleComponent, AnimationPlayer},
    root::AnimationRootBone,
};

#[derive(Component)]
pub struct AnimationTarget {
    pub id: Uuid,
    pub animator: Entity,
}

/// Builds a player's pose layout from its skeleton, once the skeleton is attached.
///
/// All bone data is sourced from existing components — the bone list and ordering from the
/// `SkeletonComponent`, the channel ids from each bone's [`AnimationTarget`], and the bind
/// pose from each bone's current `Transform` (still the rest pose at this point) — so none of
/// it is duplicated onto animation-specific types.
pub(crate) fn build_pose_layouts(
    skeletons: Query<(Entity, &SkeletonComponent), Added<SkeletonComponent>>,
    animation_targets: Query<&AnimationTarget>,
    transforms: Query<&Transform>,
    animation_players: Query<&mut AnimationPlayer>,
) {
    for (skeleton_entity, skeleton) in skeletons.iter() {
        // The player driving this skeleton is referenced by any of its animated bones.
        let Some(animator) = skeleton.bones().iter().find_map(|bone| {
            animation_targets
                .get_entity(*bone)
                .map(|target| target.animator)
        }) else {
            continue;
        };

        let Some(mut animation_player) = animation_players.get_entity(animator) else {
            continue;
        };

        let mut target_ids = Vec::with_capacity(skeleton.bones().len());
        let mut bind_pose = Vec::with_capacity(skeleton.bones().len());
        for bone in skeleton.bones() {
            target_ids.push(animation_targets.get_entity(*bone).map(|target| target.id));
            bind_pose.push(transforms.get_entity(*bone).cloned().unwrap_or_default());
        }

        animation_player.set_skeleton_entity(skeleton_entity);
        animation_player.set_layout(target_ids, bind_pose);
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

/// Writes each player's evaluated pose back onto its individual bone transforms, reading the
/// bone entities live from the `SkeletonComponent`.
pub(crate) fn apply_poses(
    animation_players: Query<&AnimationPlayer>,
    skeletons: Query<&SkeletonComponent>,
    animation_roots: Query<&AnimationRootBone>,
    transforms: Query<&mut Transform>,
) {
    for animation_player in animation_players.iter() {
        let Some(skeleton_entity) = animation_player.skeleton_entity() else {
            continue;
        };
        let Some(skeleton) = skeletons.get_entity(skeleton_entity) else {
            continue;
        };

        let pose = animation_player.current_pose();
        for (index, bone) in skeleton.bones().iter().enumerate() {
            if animation_roots.contains_entity(*bone) {
                // TODO: Accumulate root motion; leave the root bone's transform untouched.
                continue;
            }

            if let Some(mut transform) = transforms.get_entity(*bone) {
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
