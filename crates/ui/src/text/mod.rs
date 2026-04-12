use ecs::{
    command::CommandQueue,
    component::Component,
    entity::Entity,
    query::{Query, query_filter::Changed},
    resource::{Res, ResMut},
};
use glyphon::{Attrs, Buffer, Family, Metrics, Shaping, Style, Weight};
use render::components::render_entity::RenderEntity;
use window::plugin::Window;

use crate::text::resources::TextFontSystem;

pub(crate) mod resources;

/// Font family for a text node.
#[derive(Clone, Default)]
pub enum FontFamily {
    #[default]
    SansSerif,
    Serif,
    Monospace,
    /// Any font family by name (e.g. `"JetBrains Mono"`).
    Name(String),
}

/// Font style (normal or italic).
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
}

/// Text node component.
///
/// Changing any field re-builds the glyphon buffer on the next frame.
#[derive(Component)]
pub struct TextComponent {
    pub text: String,
    pub font_size: f32,
    pub line_height: f32,
    /// Font family.  Defaults to [`FontFamily::SansSerif`].
    pub font_family: FontFamily,
    /// Font weight (100–900).  400 = regular, 700 = bold.  Defaults to 400.
    pub font_weight: u16,
    /// Font style.  Defaults to [`FontStyle::Normal`].
    pub font_style: FontStyle,
}

impl Default for TextComponent {
    fn default() -> Self {
        Self {
            text: String::new(),
            font_size: 14.0,
            line_height: 18.0,
            font_family: FontFamily::default(),
            font_weight: 400,
            font_style: FontStyle::default(),
        }
    }
}

#[derive(Component)]
pub struct RenderTextComponent {
    pub(crate) buffer: glyphon::Buffer,
}

pub(crate) fn extract_added_text_nodes(
    changed_nodes: Query<(Entity, &TextComponent, Option<&RenderEntity>), Changed<TextComponent>>,
    window: Res<Window>,
    mut font_system: ResMut<TextFontSystem>,
    mut cmd: CommandQueue,
) {
    for (entity, text_component, render_entity) in changed_nodes.iter() {
        let mut text_buffer = Buffer::new(
            &mut font_system,
            Metrics {
                font_size: text_component.font_size,
                line_height: text_component.line_height,
            },
        );

        text_buffer.set_size(
            &mut font_system,
            Some(window.width() as f32),
            Some(window.height() as f32),
        );

        let family = match &text_component.font_family {
            FontFamily::SansSerif => Family::SansSerif,
            FontFamily::Serif => Family::Serif,
            FontFamily::Monospace => Family::Monospace,
            FontFamily::Name(name) => Family::Name(name.as_str()),
        };

        let style = match text_component.font_style {
            FontStyle::Normal => Style::Normal,
            FontStyle::Italic => Style::Italic,
        };

        let attrs = Attrs::new()
            .family(family)
            .weight(Weight(text_component.font_weight))
            .style(style);

        text_buffer.set_text(
            &mut font_system,
            &text_component.text,
            attrs,
            Shaping::Basic,
        );
        text_buffer.shape_until_scroll(&mut font_system, false);

        let render_text_component = RenderTextComponent {
            buffer: text_buffer,
        };

        match render_entity {
            Some(render_entity) => {
                cmd.insert(render_text_component, **render_entity);
            }
            None => {
                let render_entity = cmd.spawn(render_text_component);
                cmd.insert(RenderEntity::new(render_entity), entity);
            }
        }
    }
}
