use ecs::{
    command::CommandQueue,
    component::Component,
    entity::Entity,
    query::{Query, query_filter::Changed},
    resource::{Res, ResMut},
};
use glyphon::{Attrs, Buffer, Family, Metrics, Shaping};
use render::components::render_entity::RenderEntity;
use window::plugin::Window;

use crate::text::resources::TextFontSystem;

pub(crate) mod resources;

#[derive(Component)]
pub struct TextComponent {
    pub text: String,
    pub font_size: f32,
    pub line_height: f32,
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

        text_buffer.set_text(
            &mut font_system,
            &text_component.text,
            Attrs::new().family(Family::SansSerif),
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
