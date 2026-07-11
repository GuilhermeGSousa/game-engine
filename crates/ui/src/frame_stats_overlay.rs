use app::plugins::Plugin;
use ecs::{
    command::CommandQueue,
    component::Component,
    query::{query_filter::With, Query},
    resource::{Res, ResMut, Resource},
    system::schedule::UpdateGroup,
};
use color::LinearRgba;
use essential::time::{FrameStats, Time};

use crate::{
    material::UIMaterial,
    node::{UINode, UIRect},
    text::{FontFamily, TextComponent},
    transform::UIValue,
};

/// Marker for the overlay's text node.
#[derive(Component)]
struct FrameStatsText;

/// Seconds between overlay text refreshes. Rebuilding the glyph buffer every
/// frame would make the overlay itself a hotspot.
const REFRESH_INTERVAL: f32 = 0.25;

#[derive(Resource)]
struct OverlayRefreshTimer(f32);

/// Small always-on-top frame-time readout in the window's top-left corner.
///
/// Reads the [`FrameStats`] resource maintained by the `TimePlugin`.
/// Registered by `DefaultPlugins` (non-headless); see docs/profiling.md.
pub struct FrameStatsOverlayPlugin;

impl Plugin for FrameStatsOverlayPlugin {
    fn build(&self, app: &mut app::App) {
        app.insert_resource(OverlayRefreshTimer(0.0));
        app.add_system(UpdateGroup::Startup, spawn_overlay);
        app.add_system(UpdateGroup::Update, update_overlay_text);
    }
}

fn spawn_overlay(mut cmd: CommandQueue) {
    cmd.spawn((
        UINode {
            width: UIValue::Px(230.0),
            height: UIValue::Px(46.0),
            padding: UIRect::axes(6.0, 10.0),
            margin: UIRect::all(8.0),
            ..Default::default()
        },
        UIMaterial::flat(LinearRgba::new(0.0, 0.0, 0.0, 0.6)),
    ))
    .add_child((
        UINode {
            flex_grow: 1.0,
            ..Default::default()
        },
        TextComponent {
            text: "-- FPS".to_string(),
            font_size: 12.0,
            line_height: 16.0,
            font_family: FontFamily::Monospace,
            ..Default::default()
        },
        FrameStatsText,
    ));
}

fn update_overlay_text(
    time: Res<Time>,
    stats: Res<FrameStats>,
    mut timer: ResMut<OverlayRefreshTimer>,
    text_nodes: Query<&mut TextComponent, With<FrameStatsText>>,
) {
    timer.0 += time.delta().as_secs_f32();
    if timer.0 < REFRESH_INTERVAL || stats.is_empty() {
        return;
    }
    timer.0 = 0.0;

    for mut text in text_nodes.iter() {
        text.text = format!(
            "{:>6.0} FPS  {:>6.2} ms\np99 {:>5.2} ms  max {:>5.2} ms",
            stats.fps(),
            stats.average_ms(),
            stats.percentile_ms(0.99),
            stats.max_ms(),
        );
    }
}
