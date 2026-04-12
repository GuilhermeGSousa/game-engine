use ecs::{
    command::CommandQueue,
    resource::{Res, Resource},
};
use essential::assets::handle::AssetHandle;
use render::assets::texture::Texture;
use taffy::FlexDirection;
use ui::{
    UIViewport,
    material::UIMaterial,
    node::{UINode, UIRect},
    text::{FontFamily, TextComponent},
    transform::UIValue,
};

/// The render-target texture handle for the editor's 3D viewport.
///
/// Created by [`EditorPlugin::finish`] and inserted as a resource.  Pass
/// `rtt.0.clone()` to the editor camera's `render_target` so the camera writes
/// its output to the texture that the viewport panel reads.
#[derive(Resource)]
pub struct EditorRttHandle(pub AssetHandle<Texture>);

// ── Colours ──────────────────────────────────────────────────────────────────
const COL_TITLEBAR: [f32; 4] = [0.11, 0.11, 0.11, 1.0];
const COL_PANEL: [f32; 4] = [0.10, 0.10, 0.10, 1.0];
const COL_PANEL_HDR: [f32; 4] = [0.14, 0.14, 0.14, 1.0];
const COL_VIEWPORT_BG: [f32; 4] = [0.05, 0.05, 0.05, 1.0];
const COL_BORDER: [f32; 4] = [0.25, 0.25, 0.25, 1.0];

// ── Sizes ─────────────────────────────────────────────────────────────────────
const TITLEBAR_H: f32 = 36.0;
const PANEL_HDR_H: f32 = 28.0;
const LEFT_W: f32 = 220.0;
const RIGHT_W: f32 = 260.0;

/// Spawns the full editor chrome:
///
/// ```text
/// ┌─────────────────────────────────────────────────────┐
/// │  Title bar  ─  "Game Editor"                        │
/// ├────────────┬──────────────────────────┬─────────────┤
/// │ Hierarchy  │                          │ Properties  │
/// │  (220 px)  │     3D Viewport          │  (260 px)   │
/// │            │     (flex_grow: 1)       │             │
/// └────────────┴──────────────────────────┴─────────────┘
/// ```
pub(crate) fn spawn_editor_ui(mut cmd: CommandQueue, rtt: Res<EditorRttHandle>) {
    // ── root: full-screen column ─────────────────────────────────────────────
    let root = cmd.spawn((UINode {
        width: UIValue::Percent(1.0),
        height: UIValue::Percent(1.0),
        flex_direction: FlexDirection::Column,
        ..Default::default()
    },));

    // ── title bar ────────────────────────────────────────────────────────────
    let title_bar = cmd.spawn((
        UINode {
            width: UIValue::Percent(1.0),
            height: UIValue::Px(TITLEBAR_H),
            padding: UIRect::axes(0.0, 12.0),
            ..Default::default()
        },
        UIMaterial::with_border(COL_TITLEBAR, COL_BORDER, 1.0),
    ));
    let title_label = cmd.spawn((
        UINode {
            flex_grow: 1.0,
            ..Default::default()
        },
        TextComponent {
            text: "Game Editor".to_string(),
            font_size: 14.0,
            line_height: TITLEBAR_H,
            font_family: FontFamily::Monospace,
            font_weight: 600,
            ..Default::default()
        },
    ));
    cmd.add_child(title_bar, title_label);

    // ── main area: left panel + viewport + right panel ───────────────────────
    let main_area = cmd.spawn((UINode {
        width: UIValue::Percent(1.0),
        flex_grow: 1.0,
        flex_direction: FlexDirection::Row,
        ..Default::default()
    },));

    let left_panel = build_side_panel(&mut cmd, LEFT_W, "Hierarchy");
    let viewport = build_viewport(&mut cmd, &rtt);
    let right_panel = build_side_panel(&mut cmd, RIGHT_W, "Properties");

    cmd.add_child(main_area, left_panel);
    cmd.add_child(main_area, viewport);
    cmd.add_child(main_area, right_panel);

    cmd.add_child(root, title_bar);
    cmd.add_child(root, main_area);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_side_panel(cmd: &mut CommandQueue, width: f32, title: &str) -> ecs::entity::Entity {
    let panel = cmd.spawn((
        UINode {
            width: UIValue::Px(width),
            height: UIValue::Percent(1.0),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        },
        UIMaterial::with_border(COL_PANEL, COL_BORDER, 1.0),
    ));

    let header = cmd.spawn((
        UINode {
            width: UIValue::Percent(1.0),
            height: UIValue::Px(PANEL_HDR_H),
            padding: UIRect::axes(0.0, 10.0),
            ..Default::default()
        },
        UIMaterial::with_border(COL_PANEL_HDR, COL_BORDER, 1.0),
    ));
    let header_label = cmd.spawn((
        UINode {
            flex_grow: 1.0,
            ..Default::default()
        },
        TextComponent {
            text: title.to_string(),
            font_size: 12.0,
            line_height: PANEL_HDR_H,
            font_weight: 600,
            ..Default::default()
        },
    ));

    cmd.add_child(header, header_label);
    cmd.add_child(panel, header);
    panel
}

fn build_viewport(cmd: &mut CommandQueue, rtt: &EditorRttHandle) -> ecs::entity::Entity {
    cmd.spawn((
        UINode {
            flex_grow: 1.0,
            height: UIValue::Percent(1.0),
            ..Default::default()
        },
        UIMaterial::flat(COL_VIEWPORT_BG),
        UIViewport {
            texture: rtt.0.clone(),
        },
    ))
}
