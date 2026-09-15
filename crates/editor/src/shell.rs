//! The chrome around the scene: what is open, and what the editor last had to
//! say.
//!
//! Everything with content of its own is a panel; this is what is left.
use app::{
    schedule_groups::{LateUpdate, Startup},
    App, Plugin,
};
use ecs::{command::CommandQueue, Component, Query, Res};
use taffy::FlexDirection;
use ui::{
    material::UIMaterial,
    node::{UINode, UIRect},
    text::TextComponent,
    theme::UITheme,
    transform::UIValue,
};

use crate::dock::{DockedApp, PanelDescriptor, PanelRegistry, Region};
use crate::fonts::{glyph, icon, MEDIUM};
use crate::marks::TRANSPARENT;
use crate::project::ProjectState;
use crate::scene::SceneState;

pub const BRAND_ID: &str = "rabbithole.brand";
pub const CHATTER_ID: &str = "rabbithole.chatter";

pub struct ShellPlugin;

impl Plugin for ShellPlugin {
    fn build(&self, app: &mut App) {
        app.add_panel(PanelDescriptor {
            id: BRAND_ID,
            title: "Rabbithole",
            region: Region::Brand,
        });
        app.add_panel(PanelDescriptor {
            id: CHATTER_ID,
            title: "Chatter",
            region: Region::Foot,
        });
        app.add_system(Startup, build_chrome)
            .add_system(LateUpdate, refresh_chrome);
    }
}

#[derive(Component)]
enum Label {
    /// What is open, top-left.
    Brand,
    /// The last thing the editor said, in the foot.
    Chatter,
    /// The glyph in front of the status line: what kind of thing it is.
    ChatterGlyph,
}

fn text(theme: &UITheme, value: &str) -> TextComponent {
    TextComponent {
        text: value.into(),
        font_size: theme.font_size_md,
        line_height: theme.line_height(theme.font_size_md),
        ..Default::default()
    }
}

fn build_chrome(mut cmd: CommandQueue, registry: Res<PanelRegistry>, theme: Res<UITheme>) {
    if let Some(brand) = registry.body(BRAND_ID) {
        let row = cmd
            .spawn(UINode {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                align_items: Some(taffy::AlignItems::Center),
                gap: glam::Vec2::new(theme.spacing_sm, 0.0),
                ..Default::default()
            })
            .entity();
        cmd.add_child(brand, row);

        // The design's mark: a burrow mouth, drawn rather than lettered.
        let mark = cmd
            .spawn((
                UINode {
                    width: UIValue::Px(13.0),
                    height: UIValue::Px(19.0),
                    flex_shrink: 0.0,
                    align_items: Some(taffy::AlignItems::Center),
                    padding: UIRect {
                        top: 4.0,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                UIMaterial {
                    corner_radius: 6.0,
                    ..UIMaterial::with_border(TRANSPARENT, theme.accent, 1.5)
                },
            ))
            .entity();
        cmd.add_child(row, mark);

        let pupil = cmd
            .spawn((
                UINode {
                    width: UIValue::Px(4.0),
                    height: UIValue::Px(4.0),
                    flex_shrink: 0.0,
                    ..Default::default()
                },
                UIMaterial {
                    corner_radius: 2.0,
                    ..UIMaterial::flat(theme.accent)
                },
            ))
            .entity();
        cmd.add_child(mark, pupil);

        let wordmark = cmd
            .spawn((
                UINode::default(),
                TextComponent {
                    font_weight: MEDIUM,
                    ..text(&theme, "Rabbithole")
                },
            ))
            .entity();
        cmd.add_child(row, wordmark);

        let label = cmd
            .spawn((
                UINode::default(),
                TextComponent {
                    color: theme.text_muted,
                    ..text(&theme, "")
                },
                Label::Brand,
            ))
            .entity();
        cmd.add_child(row, label);
    }

    if let Some(chatter) = registry.body(CHATTER_ID) {
        let strip = cmd
            .spawn((
                UINode {
                    height: UIValue::Px(30.0),
                    flex_shrink: 0.0,
                    align_items: Some(taffy::AlignItems::Center),
                    padding: UIRect::axes(0.0, theme.spacing_md),
                    ..Default::default()
                }
                .clipped(),
                UIMaterial {
                    corner_radius: theme.radius_md,
                    ..UIMaterial::flat(theme.surface)
                },
            ))
            .entity();
        cmd.add_child(chatter, strip);

        let status_glyph = cmd
            .spawn((
                UINode {
                    width: UIValue::Px(18.0),
                    flex_shrink: 0.0,
                    ..Default::default()
                },
                TextComponent {
                    color: theme.text_muted,
                    ..icon(&theme, glyph::INFO, theme.font_size_md)
                },
                Label::ChatterGlyph,
            ))
            .entity();
        cmd.add_child(strip, status_glyph);

        // Text renders off a UILayout, so a label with no UINode never enters
        // the layout tree and silently draws nothing.
        let label = cmd
            .spawn((
                UINode {
                    flex_grow: 1.0,
                    ..Default::default()
                },
                TextComponent {
                    color: theme.text_muted,
                    // The foot is one line tall; a long path must cut short
                    // rather than wrap out of the strip.
                    wrap: false,
                    ellipsis: true,
                    ..text(&theme, "")
                },
                Label::Chatter,
            ))
            .entity();
        cmd.add_child(strip, label);
    }
}

fn refresh_chrome(
    project: Res<ProjectState>,
    scenes: Res<SceneState>,
    theme: Res<UITheme>,
    labels: Query<(&Label, &mut TextComponent)>,
) {
    let chatter = if project.busy() || scenes.status.is_empty() {
        project.status.clone()
    } else {
        format!("{} · {}", project.status, scenes.status)
    };
    for (label, mut component) in labels.iter() {
        let value = match label {
            Label::Brand => project
                .project
                .as_ref()
                .and_then(|open| {
                    open.root
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                })
                .unwrap_or_default(),
            Label::Chatter => chatter.clone(),
            Label::ChatterGlyph => {
                let lowered = chatter.to_lowercase();
                let warning = lowered.contains("fail") || lowered.contains("error");
                let color = if warning {
                    theme.accent
                } else {
                    theme.text_muted
                };
                if component.color != color {
                    component.color = color;
                }
                if warning { glyph::WARNING } else { glyph::INFO }.to_string()
            }
        };
        if component.text != value {
            component.text = value;
        }
    }
}
