pub mod blackboard;
pub mod clip;
pub mod evaluation;
pub mod graph;
pub mod node;
pub mod player;
pub mod plugin;
pub mod pose;
pub mod root;
pub mod target;
pub mod transition;

#[cfg(test)]
mod tests {
    use essential::assets::{asset_server::AssetServer, asset_store::AssetStore};
    use uuid::Uuid;

    use crate::blackboard::AnimationBlackboard;
    use crate::clip::{AnimationChanelOutput, AnimationChannel, AnimationClip};
    use crate::evaluation::AnimationGraphContext;
    use crate::graph::AnimationGraph;
    use crate::node::AnimationPlayMode::PlayOnce;
    use crate::node::{
        AnimationClipNode, AnimationClipNodeInstance, AnimationNode, AnimationNodeInstance,
        AnimationPlayMode,
    };

    fn unit_duration_clip() -> AnimationClip {
        let mut clip = AnimationClip::default();
        clip.add_channel(
            Uuid::new_v4(),
            AnimationChannel::new(
                vec![0.0, 1.0],
                AnimationChanelOutput::from_translation(
                    [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]].into_iter(),
                ),
            ),
        );
        clip
    }

    fn current_time(instance: &dyn AnimationNodeInstance) -> f32 {
        instance
            .as_any()
            .downcast_ref::<AnimationClipNodeInstance>()
            .unwrap()
            .current_time()
    }

    #[test]
    fn looping_clip_starts_and_wraps_at_start_time() {
        let mut server = AssetServer::new();
        let mut clips = AssetStore::<AnimationClip>::new();
        server.register_asset(&clips);
        let handle = server.add(AnimationClip::default());
        clips.insert(handle.id(), unit_duration_clip());
        let graphs = AssetStore::<AnimationGraph>::new();
        let blackboard = AnimationBlackboard::default();
        let context = AnimationGraphContext {
            animation_clips: &clips,
            animation_graphs: &graphs,
            blackboard: &blackboard,
        };

        let node = AnimationClipNode::new(handle).with_start_time(0.4);
        let mut instance = node.create_instance(&context);
        assert_eq!(current_time(instance.as_ref()), 0.4);

        instance.update(&node, 0.3, &context);
        assert!((current_time(instance.as_ref()) - 0.7).abs() < 1e-5);
        assert!(!instance.is_finished());

        instance.update(&node, 0.4, &context);
        assert_eq!(current_time(instance.as_ref()), 0.4);
        assert!(instance.is_finished());

        instance.update(&node, 0.1, &context);
        assert!((current_time(instance.as_ref()) - 0.5).abs() < 1e-5);
        assert!(!instance.is_finished());

        instance.reset();
        assert_eq!(current_time(instance.as_ref()), 0.4);
        assert!(!instance.is_finished());
    }

    #[test]
    fn play_once_clip_starts_at_start_time_and_finishes() {
        let mut server = AssetServer::new();
        let mut clips = AssetStore::<AnimationClip>::new();
        server.register_asset(&clips);
        let handle = server.add(AnimationClip::default());
        clips.insert(handle.id(), unit_duration_clip());
        let graphs = AssetStore::<AnimationGraph>::new();
        let blackboard = AnimationBlackboard::default();
        let context = AnimationGraphContext {
            animation_clips: &clips,
            animation_graphs: &graphs,
            blackboard: &blackboard,
        };

        let node = AnimationClipNode::new(handle)
            .with_play_mode(PlayOnce)
            .with_start_time(0.4);
        let mut instance = node.create_instance(&context);
        assert_eq!(current_time(instance.as_ref()), 0.4);

        instance.update(&node, 0.7, &context);
        assert!(instance.is_finished());

        instance.reset();
        assert_eq!(current_time(instance.as_ref()), 0.4);
        assert!(!instance.is_finished());
    }
}
