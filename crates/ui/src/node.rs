use std::collections::HashMap;

use ecs::{
    command::CommandQueue,
    component::Component,
    entity::{
        Entity,
        hierarchy::{ChildOf, Children},
    },
    query::{
        Query,
        query_filter::{Changed, Without},
    },
    resource::Res,
};
use glam::Vec2;
use log::warn;
use render::{components::render_entity::RenderEntity, device::RenderDevice};
use taffy::{AvailableSpace, Dimension, FlexDirection, NodeId, Size, Style, TaffyTree};
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
    pub flex_direction: FlexDirection,
    pub flex_grow: f32,
    pub flex_shrink: f32,
}

impl UINode {
    fn size(&self) -> Size<Dimension> {
        Size {
            width: match self.width {
                UIValue::Auto => Dimension::auto(),
                UIValue::Px(value) => Dimension::length(value),
                UIValue::Percent(value) => Dimension::percent(value),
            },
            height: match self.height {
                UIValue::Auto => Dimension::auto(),
                UIValue::Px(value) => Dimension::length(value),
                UIValue::Percent(value) => Dimension::percent(value),
            },
        }
    }

    fn style(&self) -> Style {
        Style {
            size: self.size(),
            flex_direction: self.flex_direction,
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            ..Default::default()
        }
    }
}

impl Default for UINode {
    fn default() -> Self {
        Self {
            width: Default::default(),
            height: Default::default(),
            flex_direction: Default::default(),
            flex_grow: 0.0,
            flex_shrink: 1.0,
        }
    }
}

// Post taffy computed layout data
#[derive(Component)]
pub(crate) struct UIComputedNode {
    pub(crate) location: Vec2,
    pub(crate) size: Vec2,
    pub(crate) z_index: i32,
}

#[derive(Component)]
pub(crate) struct RenderUINode {
    pub(crate) index_buffer: Buffer,
    pub(crate) index_count: u32,
    pub(crate) vertex_buffer: Buffer,
    pub(crate) material_bind_group: wgpu::BindGroup,
    pub(crate) location: Vec2,
    pub(crate) size: Vec2,
    pub(crate) z_index: i32,
}

pub(crate) fn compute_ui_nodes(
    ui_nodes: Query<(Entity, &UINode, Option<&Children>)>,
    ui_roots: Query<(Entity, &UINode, Option<&Children>), Without<ChildOf>>,
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

    let mut entity_to_taffy = HashMap::new();
    let mut entity_to_z_index = HashMap::new();

    for (entity, node, children) in ui_roots.iter() {
        let node_id = match taffy.new_leaf(node.style()) {
            Ok(node_id) => node_id,
            Err(error) => {
                warn!("Error adding UI node: {}", error);
                continue;
            }
        };
        let z_index = 0;
        entity_to_taffy.insert(entity, node_id);
        entity_to_z_index.insert(entity, z_index);
        if let Some(children) = children {
            generate_taffy_children_recursive(
                &mut taffy,
                node_id,
                z_index,
                children,
                &ui_nodes,
                &mut entity_to_taffy,
                &mut entity_to_z_index,
            );
        }

        if let Err(error) = taffy.compute_layout(node_id, window_size) {
            warn!("Error computing UI layout: {}", error);
            continue;
        }
    }

    for (node_entity, _, _) in ui_nodes.iter() {
        let Some(node_id) = entity_to_taffy.get(&node_entity) else {
            continue;
        };

        let Some(z_index) = entity_to_z_index.get(&node_entity) else {
            continue;
        };

        let node_layout = match taffy.layout(*node_id) {
            Ok(layout) => layout,
            Err(error) => {
                warn!("Error computing UI node: {}", error);
                continue;
            }
        };

        cmd.insert(
            UIComputedNode {
                location: Vec2::new(node_layout.location.x, node_layout.location.y),
                size: Vec2::new(node_layout.size.width, node_layout.size.height),
                z_index: *z_index,
            },
            node_entity,
        );
    }
}

fn generate_taffy_children_recursive(
    taffy: &mut TaffyTree<()>,
    parent_node: NodeId,
    current_z_index: i32,
    children: &Children,
    ui_nodes: &Query<(Entity, &UINode, Option<&Children>)>,
    entity_to_taffy: &mut HashMap<Entity, NodeId>,
    entity_to_z_index: &mut HashMap<Entity, i32>,
) {
    let next_z_index = current_z_index + 1;
    for child in children {
        let Some((_, child_node, grand_children)) = ui_nodes.get_entity(*child) else {
            continue;
        };

        let Ok(child_node_id) = taffy.new_leaf(child_node.style()) else {
            continue;
        };

        if !taffy.add_child(parent_node, child_node_id).is_ok() {
            continue;
        }

        entity_to_taffy.insert(*child, child_node_id);
        entity_to_z_index.insert(*child, next_z_index);
        if let Some(grand_children) = grand_children {
            generate_taffy_children_recursive(
                taffy,
                child_node_id,
                next_z_index,
                grand_children,
                ui_nodes,
                entity_to_taffy,
                entity_to_z_index,
            );
        }
    }
}

pub(crate) fn extract_added_ui_nodes(
    computed_nodes: Query<
        (
            Entity,
            &UIComputedNode,
            &UIMaterialComponent,
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
        let color = node_material.color;

        let color_array = [
            color.r as f32,
            color.g as f32,
            color.b as f32,
            color.a as f32,
        ];

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
                pos_coords: [computed_node.location.x, computed_node.location.y],
            },
            UIVertex {
                pos_coords: [
                    computed_node.location.x,
                    computed_node.location.y + computed_node.size.y,
                ],
            },
            UIVertex {
                pos_coords: (computed_node.location + computed_node.size).to_array(),
            },
            UIVertex {
                pos_coords: [
                    computed_node.location.x + computed_node.size.x,
                    computed_node.location.y,
                ],
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
            location: computed_node.location,
            size: computed_node.size,
            z_index: computed_node.z_index,
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
