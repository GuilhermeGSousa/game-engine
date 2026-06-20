use app::plugins::Plugin;
use ecs::system::schedule::UpdateGroup;

use crate::{
    clip::AnimationClip,
    graph::AnimationGraph,
    target::{
        apply_poses, build_pose_layouts, evaluate_animations, initialize_animation_players,
        update_animation_players,
    },
};

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_asset::<AnimationClip>();
        app.register_asset::<AnimationGraph>();

        // Registration order matters: every system below touches `AnimationPlayer`, so the
        // scheduler's access-conflict edges run them in this order — layout, init, advance
        // time, evaluate the full pose, then apply it to the bone transforms.
        app.add_system(UpdateGroup::LateUpdate, build_pose_layouts)
            .add_system(UpdateGroup::LateUpdate, initialize_animation_players)
            .add_system(UpdateGroup::LateUpdate, update_animation_players)
            .add_system(UpdateGroup::LateUpdate, evaluate_animations)
            .add_system(UpdateGroup::LateUpdate, apply_poses);
    }
}
