use ecs::{
    command::CommandQueue,
    component::Component,
    entity::{Entity, hierarchy::Children},
    query::{Query, query_filter::Changed},
    resource::Res,
};
use glam::Vec2;
use log::warn;
use render::{components::render_entity::RenderEntity, device::RenderDevice};
use taffy::{
    AvailableSpace, Dimension, Size, Style, TaffyTree,
    prelude::{FromLength, FromPercent},
};
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, Buffer, BufferUsages,
    util::{BufferInitDescriptor, DeviceExt},
};
use window::plugin::Window;

use crate::{
    layout::UIMaterialLayout,
    material::UIMaterialComponent,
    transform::UIValue,
    vertex::{QUAD_INDICES, UIVertex},
};

// User defined layout data
#[derive(Component)]
pub struct UINode {
    pub width: UIValue,
    pub height: UIValue,
}

impl UINode {
    fn size(&self) -> Size<Dimension> {
        Size {
            width: match self.width {
                UIValue::Auto => Dimension::auto(),
                UIValue::Px(value) => Dimension::from_length(value),
                UIValue::Percent(value) => Dimension::from_percent(value),
            },
            height: match self.height {
                UIValue::Auto => Dimension::auto(),
                UIValue::Px(value) => Dimension::from_length(value),
                UIValue::Percent(value) => Dimension::from_percent(value),
            },
        }
    }
}

// Post taffy computed layout data
#[derive(Component)]
pub(crate) struct UIComputedNode {
    pub(crate) size: Vec2,
}

#[derive(Component)]
pub(crate) struct RenderUINode {
    pub(crate) index_buffer: Buffer,
    pub(crate) index_count: u32,
    pub(crate) vertex_buffer: Buffer,
    pub(crate) material_bind_group: wgpu::BindGroup,
}

pub(crate) fn compute_ui_nodes(
    ui_nodes: Query<(Entity, &UINode, Option<&Children>), Changed<UINode>>,
    window: Res<Window>,
    mut cmd: CommandQueue,
) {
    let mut taffy: TaffyTree<()> = TaffyTree::new();

    let window_width = window.width() as f32;
    let window_height = window.height() as f32;

    let window_size = Size {
        width: AvailableSpace::Definite(window_width),
        height: AvailableSpace::Definite(window_height),
    };

    let root_node = taffy
        .new_leaf(Style {
            size: Size {
                width: Dimension::from_length(window_width),
                height: Dimension::from_length(window_height),
            },
            ..Default::default()
        })
        .expect("Error creating root UI node");

    for (entity, node, children) in ui_nodes.iter() {
        let style = Style {
            size: node.size(),
            ..Default::default()
        };

        let node_id = match taffy.new_leaf(style) {
            Ok(node_id) => node_id,
            Err(error) => {
                warn!("Error adding UI node: {}", error);
                continue;
            }
        };

        if let Err(error) = taffy.compute_layout(node_id, window_size) {
            warn!("Error computing UI node: {}", error);
            continue;
        }

        let node_layout = match taffy.layout(node_id) {
            Ok(layout) => layout,
            Err(error) => {
                warn!("Error computing UI node: {}", error);
                continue;
            }
        };

        println!(
            "x: {} y: {}",
            node_layout.location.x, node_layout.location.y
        );
        println!(
            "width: {} height: {}",
            node_layout.size.width, node_layout.size.height
        );
        cmd.insert(
            UIComputedNode {
                size: Vec2::new(
                    node_layout.size.width / 100.0,
                    node_layout.size.height / 100.0,
                ),
            },
            entity,
        );
    }
}

pub(crate) fn extract_added_ui_nodes(
    computed_nodes: Query<
        (
            Entity,
            &UIComputedNode,
            Option<&UIMaterialComponent>,
            Option<&RenderEntity>,
        ),
        Changed<UIComputedNode>,
    >,
    ui_material_layout: Res<UIMaterialLayout>,
    device: Res<RenderDevice>,
    mut cmd: CommandQueue,
) {
    for (computed_node_entity, computed_node, node_material, render_entity) in computed_nodes.iter()
    {
        if node_material.is_none() {
            warn!("UI Node missing a material");
        }
        let color = node_material.map(|m| m.color).unwrap_or(wgpu::Color::GREEN);

        let color_array = [color.r, color.g, color.b, color.a];

        let material_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("UI Material Buffer"),
            contents: bytemuck::cast_slice(&color_array),
            usage: BufferUsages::UNIFORM,
        });

        let material_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("UI Material Bind Group"),
            layout: &ui_material_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: material_buffer.as_entire_binding(),
            }],
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI Index Buffer"),
            contents: bytemuck::cast_slice(&QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let vertices = [
            UIVertex {
                pos_coords: [0.0, 0.0],
            },
            UIVertex {
                pos_coords: [0.0, computed_node.size.y],
            },
            UIVertex {
                pos_coords: computed_node.size.to_array(),
            },
            UIVertex {
                pos_coords: [computed_node.size.x, 0.0],
            },
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let render_ui_node = RenderUINode {
            index_buffer: index_buffer,
            index_count: QUAD_INDICES.len() as u32,
            vertex_buffer: vertex_buffer,
            material_bind_group,
        };

        match render_entity {
            Some(render_entity) => {
                cmd.insert(render_ui_node, **render_entity);
            }
            None => {
                let render_entity = cmd.spawn(render_ui_node);
                cmd.insert(RenderEntity::new(render_entity), computed_node_entity);
            }
        }
    }
}
