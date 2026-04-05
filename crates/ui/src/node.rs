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
use render::{
    assets::material::AsBindGroup,
    components::render_entity::RenderEntity,
    device::RenderDevice,
    render_asset::{
        RenderAssets,
        render_texture::{DummyRenderTexture, RenderTexture},
    },
};
use taffy::{
    AlignItems, AvailableSpace, Dimension, FlexDirection, JustifyContent, LengthPercentage,
    LengthPercentageAuto, NodeId, Rect, Size, Style, TaffyTree,
};
use wgpu::{Buffer, util::DeviceExt};
use window::plugin::Window;

use crate::{
    material::UIMaterial,
    transform::UIValue,
    vertex::{QUAD_INDICES, UIVertex},
};

/// Converts a [`UIValue`] to a Taffy [`LengthPercentage`].
///
/// `Auto` is not supported by `LengthPercentage` (which is used for padding
/// and similar properties where CSS does not allow `auto`). It is therefore
/// mapped to zero length, which leaves the space unset.
fn ui_value_to_length_percentage(value: &UIValue) -> LengthPercentage {
    match value {
        UIValue::Auto => LengthPercentage::length(0.0),
        UIValue::Px(v) => LengthPercentage::length(*v),
        UIValue::Percent(v) => LengthPercentage::percent(*v),
    }
}

fn ui_value_to_length_percentage_auto(value: &UIValue) -> LengthPercentageAuto {
    match value {
        UIValue::Auto => LengthPercentageAuto::auto(),
        UIValue::Px(v) => LengthPercentageAuto::length(*v),
        UIValue::Percent(v) => LengthPercentageAuto::percent(*v),
    }
}

/// Per-side spacing values (top, right, bottom, left).
#[derive(Default)]
pub struct UISides {
    pub top: UIValue,
    pub right: UIValue,
    pub bottom: UIValue,
    pub left: UIValue,
}

impl UISides {
    pub fn all(value: UIValue) -> Self
    where
        UIValue: Clone,
    {
        Self {
            top: value.clone(),
            right: value.clone(),
            bottom: value.clone(),
            left: value,
        }
    }

    pub fn axes(horizontal: UIValue, vertical: UIValue) -> Self
    where
        UIValue: Clone,
    {
        Self {
            top: vertical.clone(),
            right: horizontal.clone(),
            bottom: vertical,
            left: horizontal,
        }
    }
}

// User defined layout data
#[derive(Component)]
pub struct UINode {
    pub width: UIValue,
    pub height: UIValue,
    pub flex_direction: FlexDirection,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    /// Inner spacing between the node border and its children.
    pub padding: UISides,
    /// Outer spacing between this node and its siblings.
    pub margin: UISides,
    /// How children are aligned along the cross axis.
    pub align_items: Option<AlignItems>,
    /// How children are distributed along the main axis.
    pub justify_content: Option<JustifyContent>,
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
            padding: Rect {
                top: ui_value_to_length_percentage(&self.padding.top),
                right: ui_value_to_length_percentage(&self.padding.right),
                bottom: ui_value_to_length_percentage(&self.padding.bottom),
                left: ui_value_to_length_percentage(&self.padding.left),
            },
            margin: Rect {
                top: ui_value_to_length_percentage_auto(&self.margin.top),
                right: ui_value_to_length_percentage_auto(&self.margin.right),
                bottom: ui_value_to_length_percentage_auto(&self.margin.bottom),
                left: ui_value_to_length_percentage_auto(&self.margin.left),
            },
            align_items: self.align_items,
            justify_content: self.justify_content,
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
            padding: Default::default(),
            margin: Default::default(),
            align_items: None,
            justify_content: None,
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
    pub(crate) location: Vec2,
    pub(crate) size: Vec2,
    pub(crate) z_index: i32,
}

#[derive(Component)]
pub(crate) struct RenderUIMaterial {
    pub(crate) material_bind_group: wgpu::BindGroup,
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

        if taffy.add_child(parent_node, child_node_id).is_err() {
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
        (Entity, &UIComputedNode, Option<&RenderEntity>),
        Changed<UIComputedNode>,
    >,
    device: Res<RenderDevice>,
    window: Res<Window>,
    mut cmd: CommandQueue,
) {
    let win_w = window.width() as f32;
    let win_h = window.height() as f32;

    // Convert pixel coordinates to Normalized Device Coordinates (NDC).
    // Screen space: (0,0) = top-left corner, Y increases downward.
    // NDC space:    (-1,-1) = bottom-left, (+1,+1) = top-right, Y increases upward.
    let to_ndc = |px: f32, py: f32| -> [f32; 2] {
        [
            (px / win_w) * 2.0 - 1.0, // map [0, width]  → [-1, +1]
            1.0 - (py / win_h) * 2.0, // map [0, height] → [+1, -1] (flip Y)
        ]
    };

    for (computed_node_entity, computed_node, render_entity) in computed_nodes.iter() {
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI Index Buffer"),
            contents: bytemuck::cast_slice(&QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let x = computed_node.location.x;
        let y = computed_node.location.y;
        let w = computed_node.size.x;
        let h = computed_node.size.y;

        let vertices = [
            UIVertex {
                pos_coords: to_ndc(x, y),
            }, // top-left
            UIVertex {
                pos_coords: to_ndc(x, y + h),
            }, // bottom-left
            UIVertex {
                pos_coords: to_ndc(x + w, y + h),
            }, // bottom-right
            UIVertex {
                pos_coords: to_ndc(x + w, y),
            }, // top-right
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let render_ui_node = RenderUINode {
            index_buffer,
            index_count: QUAD_INDICES.len() as u32,
            vertex_buffer,
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

/// Extracts [`UIMaterial`] changes into GPU-side [`RenderUIMaterial`] bind groups.
///
/// The bind group is created via [`UIMaterial::create_bind_group`] — the same
/// macro-generated method that is used to verify bind-group layout compatibility
/// — so the layout used here is always consistent with the one used to build the
/// UI render pipeline.
pub(crate) fn extract_added_ui_materials(
    computed_nodes: Query<(Entity, &UIMaterial, Option<&RenderEntity>), Changed<UIMaterial>>,
    device: Res<RenderDevice>,
    render_textures: Res<RenderAssets<RenderTexture>>,
    dummy_texture: Res<DummyRenderTexture>,
    ui_pipeline: Res<render::MaterialPipeline<UIMaterial>>,
    mut cmd: CommandQueue,
) {
    for (computed_node_entity, node_material, render_entity) in computed_nodes.iter() {
        let Ok(material_bind_group) = node_material.create_bind_group(
            &device,
            &render_textures,
            &dummy_texture,
            &ui_pipeline.bind_group_layout,
        ) else {
            continue;
        };

        let render_ui_material = RenderUIMaterial {
            material_bind_group,
        };

        match render_entity {
            Some(render_entity) => {
                cmd.insert(render_ui_material, **render_entity);
            }
            None => {
                let render_entity = cmd.spawn(render_ui_material);
                cmd.insert(RenderEntity::new(render_entity), computed_node_entity);
            }
        }
    }
}
