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
    animation_players: Query<(&mut AnimationPlayer, &AnimationHandleComponent)>,
    animation_targets: Query<(&mut Transform, &AnimationTarget)>,
    animation_clips: Res<AssetStore<AnimationClip>>,
    time: Res<Time>,
) {
    for (mut target_transform, animation_target) in animation_targets.iter() {
        let Some((mut animation_player, animation_handle)) =
            animation_players.get_entity(animation_target.animator)
        else {
            continue;
        };

        let Some(animation_clip) = animation_clips.get(&animation_handle) else {
            continue;
        };

        // Find the channel for this animation target
        let Some(animation_channels) = animation_clip.get_channels(&animation_target.id) else {
            continue;
        };

        // Based on the current time of the animation player + delta time, interpolate the target's transform
        for animation_channel in animation_channels {
            animation_channel.interpolate(animation_player.current_time(), &mut target_transform);
        }

        // Update animation player
        animation_player.update(time.delta().as_secs_f32());
    }
}
