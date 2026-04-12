use derive_more::Deref;
use ecs::{
    command::CommandQueue,
    entity::Entity,
    resource::{Res, Resource},
};
use essential::assets::handle::AssetHandle;
use render::assets::texture::Texture;
use taffy::FlexDirection;
use ui::{
    UIViewport,
    checkbox::UICheckbox,
    interaction::{Interactable, UIInteractionStyle},
    material::UIMaterial,
    node::{UINode, UIRect},
    slider::UISlider,
    text::{FontFamily, TextComponent},
    text_input::UITextInput,
    transform::UIValue,
};

/// The render-target texture handle for the editor's 3D viewport.
#[derive(Resource, Deref)]
pub struct EditorRttHandle(pub AssetHandle<Texture>);

// ── Colours ───────────────────────────────────────────────────────────────────
const COL_TITLEBAR: [f32; 4] = [0.11, 0.11, 0.11, 1.0];
const COL_PANEL: [f32; 4] = [0.10, 0.10, 0.10, 1.0];
const COL_PANEL_HDR: [f32; 4] = [0.14, 0.14, 0.14, 1.0];
const COL_VIEWPORT_BG: [f32; 4] = [0.05, 0.05, 0.05, 1.0];
const COL_BORDER: [f32; 4] = [0.25, 0.25, 0.25, 1.0];
const COL_SLIDER_TRACK: [f32; 4] = [0.18, 0.18, 0.18, 1.0];
const COL_INPUT_BG: [f32; 4] = [0.13, 0.13, 0.13, 1.0];

// ── Sizes ─────────────────────────────────────────────────────────────────────
const TITLEBAR_H: f32 = 36.0;
const PANEL_HDR_H: f32 = 28.0;
const LEFT_W: f32 = 220.0;
const RIGHT_W: f32 = 260.0;
const ROW_H: f32 = 22.0;
const SLIDER_H: f32 = 16.0;
const CHECKBOX_SIZE: f32 = 16.0;
const LABEL_W: f32 = 76.0;

/// Spawns the full editor chrome:
///
/// ```text
/// ┌─────────────────────────────────────────────────────┐
/// │  Title bar  ─  "Game Editor"                        │
/// ├────────────┬──────────────────────────┬─────────────┤
/// │ Hierarchy  │                          │ Properties  │
/// │  (220 px)  │     3D Viewport          │  (260 px)   │
/// │            │     (flex_grow: 1)       │  [sliders]  │
/// │            │                          │  [checks]   │
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
    let right_panel = build_properties_panel(&mut cmd);

    cmd.add_child(main_area, left_panel);
    cmd.add_child(main_area, viewport);
    cmd.add_child(main_area, right_panel);

    cmd.add_child(root, title_bar);
    cmd.add_child(root, main_area);
}

// ── Panel builders ────────────────────────────────────────────────────────────

fn build_side_panel(cmd: &mut CommandQueue, width: f32, title: &str) -> Entity {
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

fn build_viewport(cmd: &mut CommandQueue, rtt: &EditorRttHandle) -> Entity {
    cmd.spawn((
        UINode {
            flex_grow: 1.0,
            height: UIValue::Percent(1.0),
            ..Default::default()
        },
        UIMaterial::flat(COL_VIEWPORT_BG),
        UIViewport {
            texture: (*rtt).clone(),
        },
    ))
}

/// Builds the right Properties panel, pre-populated with placeholder widgets
/// so the slider, checkbox, and text-input components can all be exercised at
/// runtime without needing a real scene object selected.
///
/// Layout:
/// ```text
/// ┌─ Properties ──────────────────┐
/// │ ─ Name ─                      │
/// │ [Entity name...         ]     │
/// │ ─ Transform ─                 │
/// │ X  [══════|══════════════]    │
/// │ Y  [══════════|══════════]    │
/// │ Z  [════|════════════════]    │
/// │ ─ Light ─                     │
/// │ Intensity [══════════════|══] │
/// │ ─ Flags ─                     │
/// │ [✓] Visible                   │
/// │ [ ] Cast Shadows              │
/// └───────────────────────────────┘
/// ```
fn build_properties_panel(cmd: &mut CommandQueue) -> Entity {
    let panel = cmd.spawn((
        UINode {
            width: UIValue::Px(RIGHT_W),
            height: UIValue::Percent(1.0),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        },
        UIMaterial::with_border(COL_PANEL, COL_BORDER, 1.0),
    ));

    // Header
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
            text: "Properties".to_string(),
            font_size: 12.0,
            line_height: PANEL_HDR_H,
            font_weight: 600,
            ..Default::default()
        },
    ));
    cmd.add_child(header, header_label);
    cmd.add_child(panel, header);

    // Scrollable content column
    let content = cmd.spawn((UINode {
        width: UIValue::Percent(1.0),
        flex_grow: 1.0,
        flex_direction: FlexDirection::Column,
        padding: UIRect::axes(6.0, 8.0),
        ..Default::default()
    },));

    // ── Name ─────────────────────────────────────────────────────────────────
    add_section_label(cmd, content, "Name");
    let name_input = spawn_text_input(cmd, "Entity name...");
    cmd.add_child(content, name_input);

    // ── Transform ────────────────────────────────────────────────────────────
    add_section_label(cmd, content, "Transform");
    for (label, value) in [("X", 0.0_f32), ("Y", 2.0), ("Z", -6.0)] {
        let slider = spawn_slider(cmd, value, -10.0, 10.0);
        let row = spawn_label_row(cmd, label, slider);
        cmd.add_child(content, row);
    }

    // ── Light ─────────────────────────────────────────────────────────────────
    add_section_label(cmd, content, "Light");
    let intensity_slider = spawn_slider(cmd, 15.0, 0.0, 20.0);
    let intensity_row = spawn_label_row(cmd, "Intensity", intensity_slider);
    cmd.add_child(content, intensity_row);

    // ── Flags ─────────────────────────────────────────────────────────────────
    add_section_label(cmd, content, "Flags");
    let vis_row = spawn_checkbox_row(cmd, "Visible", true);
    let shadow_row = spawn_checkbox_row(cmd, "Cast Shadows", false);
    cmd.add_child(content, vis_row);
    cmd.add_child(content, shadow_row);

    cmd.add_child(panel, content);
    panel
}

// ── Widget helpers ────────────────────────────────────────────────────────────

/// Appends a small section-divider label (e.g. `"─ Transform ─"`) into `parent`.
fn add_section_label(cmd: &mut CommandQueue, parent: Entity, text: &str) {
    let label = cmd.spawn((
        UINode {
            width: UIValue::Percent(1.0),
            height: UIValue::Px(18.0),
            margin: UIRect {
                top: 6.0,
                bottom: 2.0,
                ..Default::default()
            },
            ..Default::default()
        },
        TextComponent {
            text: format!("─ {text} ─"),
            font_size: 10.0,
            line_height: 18.0,
            font_weight: 600,
            ..Default::default()
        },
    ));
    cmd.add_child(parent, label);
}

/// Returns a horizontal row entity with a fixed-width label on the left and
/// `widget` on the right (flex_grow: 1).
fn spawn_label_row(cmd: &mut CommandQueue, label: &str, widget: Entity) -> Entity {
    let row = cmd.spawn((UINode {
        width: UIValue::Percent(1.0),
        height: UIValue::Px(ROW_H),
        flex_direction: FlexDirection::Row,
        margin: UIRect {
            bottom: 4.0,
            ..Default::default()
        },
        ..Default::default()
    },));

    let label_entity = cmd.spawn((
        UINode {
            width: UIValue::Px(LABEL_W),
            height: UIValue::Percent(1.0),
            ..Default::default()
        },
        TextComponent {
            text: label.to_string(),
            font_size: 11.0,
            line_height: ROW_H,
            ..Default::default()
        },
    ));

    cmd.add_child(row, label_entity);
    cmd.add_child(row, widget);
    row
}

/// Spawns a draggable slider track entity.
///
/// The fill child is automatically added by `setup_slider_visuals` on the first
/// frame.  `UIInteractionStyle` gives the track a subtle hover/press tint.
fn spawn_slider(cmd: &mut CommandQueue, value: f32, min: f32, max: f32) -> Entity {
    cmd.spawn((
        UINode {
            flex_grow: 1.0,
            height: UIValue::Px(SLIDER_H),
            margin: UIRect {
                top: (ROW_H - SLIDER_H) / 2.0,
                ..Default::default()
            },
            ..Default::default()
        },
        UIMaterial::with_border(COL_SLIDER_TRACK, COL_BORDER, 1.0),
        UISlider::new(value, min, max),
        Interactable,
        UIInteractionStyle {
            normal: COL_SLIDER_TRACK,
            hovered: [0.22, 0.22, 0.22, 1.0],
            pressed: [0.16, 0.16, 0.16, 1.0],
            disabled: [0.12, 0.12, 0.12, 0.5],
        },
    ))
}

/// Spawns a single-line text input that spans the full content width.
fn spawn_text_input(cmd: &mut CommandQueue, placeholder: &str) -> Entity {
    cmd.spawn((
        UINode {
            width: UIValue::Percent(1.0),
            height: UIValue::Px(ROW_H),
            padding: UIRect::axes(0.0, 6.0),
            margin: UIRect {
                bottom: 4.0,
                ..Default::default()
            },
            ..Default::default()
        },
        UIMaterial::with_border(COL_INPUT_BG, COL_BORDER, 1.0),
        TextComponent {
            text: placeholder.to_string(),
            font_size: 11.0,
            line_height: ROW_H,
            ..Default::default()
        },
        UITextInput::new(placeholder),
        Interactable,
    ))
}

/// Spawns a row containing a checkbox square followed by a text label.
fn spawn_checkbox_row(cmd: &mut CommandQueue, label: &str, checked: bool) -> Entity {
    let row = cmd.spawn((UINode {
        width: UIValue::Percent(1.0),
        height: UIValue::Px(ROW_H),
        flex_direction: FlexDirection::Row,
        margin: UIRect {
            bottom: 4.0,
            ..Default::default()
        },
        ..Default::default()
    },));

    let checkbox = cmd.spawn((
        UINode {
            width: UIValue::Px(CHECKBOX_SIZE),
            height: UIValue::Px(CHECKBOX_SIZE),
            margin: UIRect {
                top: (ROW_H - CHECKBOX_SIZE) / 2.0,
                right: 6.0,
                ..Default::default()
            },
            ..Default::default()
        },
        UIMaterial::with_border(
            if checked {
                [0.20, 0.50, 0.90, 1.0]
            } else {
                [0.12, 0.12, 0.12, 1.0]
            },
            COL_BORDER,
            1.0,
        ),
        UICheckbox::new(checked),
        Interactable,
    ));

    let label_entity = cmd.spawn((
        UINode {
            flex_grow: 1.0,
            height: UIValue::Percent(1.0),
            ..Default::default()
        },
        TextComponent {
            text: label.to_string(),
            font_size: 11.0,
            line_height: ROW_H,
            ..Default::default()
        },
    ));

    cmd.add_child(row, checkbox);
    cmd.add_child(row, label_entity);
    row
}
