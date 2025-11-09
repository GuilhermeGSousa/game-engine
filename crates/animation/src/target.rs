use ecs::{component::Component, entity::Entity, query::Query, resource::Res};
use essential::{assets::asset_store::AssetStore, time::Time, transform::Transform};
use uuid::Uuid;

use crate::{
    clip::AnimationClip,
    player::{AnimationHandleComponent, AnimationPlayer},
};

#[derive(Component)]
pub struct AnimationTarget {
    pub id: Uuid,
    pub animator: Entity,
}

pub(crate) fn animate_targets(
    animation_players: Query<(&AnimationPlayer, &AnimationHandleComponent)>,
    animation_targets: Query<(&mut Transform, &AnimationTarget)>,
    animation_clips: Res<AssetStore<AnimationClip>>,
    time: Res<Time>,
) {
    for (target_transform, animation_target) in animation_targets.iter() {
        let Some((animation_player, animation_handle)) =
            animation_players.get_entity(animation_target.animator)
        else {
            continue;
        };

        let Some(animation_clip) = animation_clips.get(&animation_handle) else {
            continue;
        };

        // Find the channel for this animation target
        let Some(animation_channel) = animation_clip.get_channel(&animation_target.id) else {
            continue;
        };

        // Based on the current time of the animation player + delta time, interpolate the target's transform

        // Update animation player
    }
}
