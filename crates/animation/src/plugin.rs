use app::plugins::Plugin;

use crate::{clip::AnimationClip, target::animate_targets};

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_asset::<AnimationClip>();

        app.add_system(app::update_group::UpdateGroup::LateUpdate, animate_targets);
    }
}
