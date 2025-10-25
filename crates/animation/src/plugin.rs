use app::plugins::Plugin;

use crate::clip::AnimationClip;

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_asset::<AnimationClip>();
    }
}
