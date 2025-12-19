use app::plugins::Plugin;

use crate::{
    clip::AnimationClip,
    graph::AnimationGraph,
    target::{animate_targets, initialize_animation_players, update_animation_players},
};

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_asset::<AnimationClip>();
        app.register_asset::<AnimationGraph>();

        app.add_system(app::update_group::UpdateGroup::LateUpdate, animate_targets)
            .add_system(
                app::update_group::UpdateGroup::LateUpdate,
                initialize_animation_players,
            )
            .add_system(
                app::update_group::UpdateGroup::LateUpdate,
                update_animation_players,
            );
    }
}
