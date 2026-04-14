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
    resource::{Res, Resource},
};
use essential::assets::handle::AssetHandle;
use glam::Vec2;
use log::warn;
use render::{
    assets::{material::AsBindGroup, texture::Texture},
    components::{
        camera::{Camera, RenderCamera, RenderTarget},
        render_entity::RenderEntity,
    },
    device::RenderDevice,
    render_asset::{
        RenderAssets,
        render_texture::{DummyRenderTexture, RenderTexture},
    },
};
use taffy::{
    AvailableSpace, Dimension, FlexDirection, LengthPercentage, LengthPercentageAuto, NodeId, Rect,
    Size, Style, TaffyTree,
};
use wgpu::{Buffer, util::DeviceExt};
use window::plugin::Window;

use crate::{
    material::UIMaterial,
    transform::UIValue,
    vertex::{QUAD_INDICES, UIVertex},
};

/// A uniform padding/margin value for one or all sides of a UI node (in pixels).
#[derive(Default, Clone, Copy)]
pub struct UIRect {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl UIRect {
    /// Applies the same value to all four sides.
    pub fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Applies `vertical` to top/bottom and `horizontal` to left/right.
    pub fn axes(vertical: f32, horizontal: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    fn to_taffy_padding(self) -> Rect<LengthPercentage> {
        Rect {
            top: LengthPercentage::length(self.top),
            right: LengthPercentage::length(self.right),
            bottom: LengthPercentage::length(self.bottom),
            left: LengthPercentage::length(self.left),
        }
    }

    fn to_taffy_margin(self) -> Rect<LengthPercentageAuto> {
        Rect {
            top: LengthPercentageAuto::length(self.top),
            right: LengthPercentageAuto::length(self.right),
            bottom: LengthPercentageAuto::length(self.bottom),
            left: LengthPercentageAuto::length(self.left),
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
    pub padding: UIRect,
    pub margin: UIRect,
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
            padding: self.padding.to_taffy_padding(),
            margin: self.margin.to_taffy_margin(),
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
            padding: UIRect::default(),
            margin: UIRect::default(),
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

    let window_size = Size {
        width: AvailableSpace::Definite(window.width() as f32),
        height: AvailableSpace::Definite(window.height() as f32),
    };

    // Phase 1: build the Taffy tree mirroring the ECS hierarchy.
    let mut entity_to_taffy: HashMap<Entity, NodeId> = HashMap::new();

    for (entity, node, children) in ui_roots.iter() {
        let Ok(node_id) = taffy.new_leaf(node.style()) else {
            warn!("Error adding root UI node");
            continue;
        };
        entity_to_taffy.insert(entity, node_id);

        if let Some(children) = children {
            build_taffy_tree(
                &mut taffy,
                node_id,
                children,
                &ui_nodes,
                &mut entity_to_taffy,
            );
        }

        if let Err(error) = taffy.compute_layout(node_id, window_size) {
            warn!("Error computing UI layout: {}", error);
        }
    }

    // Phase 2: walk each root and accumulate absolute screen positions as we
    // descend.  Taffy's `layout().location` is relative to the parent, so we
    // must add the parent's absolute position at every level.
    for (entity, _, children) in ui_roots.iter() {
        let Some(&node_id) = entity_to_taffy.get(&entity) else {
            continue;
        };
        let Ok(layout) = taffy.layout(node_id) else {
            continue;
        };

        let abs_pos = Vec2::new(layout.location.x, layout.location.y);
        let size = Vec2::new(layout.size.width, layout.size.height);

        cmd.insert(
            UIComputedNode {
                location: abs_pos,
                size,
                z_index: 0,
            },
            entity,
        );

        if let Some(children) = children {
            write_absolute_positions(
                &taffy,
                abs_pos,
                0,
                children,
                &ui_nodes,
                &entity_to_taffy,
                &mut cmd,
            );
        }
    }
}

/// Recursively registers every descendant as a Taffy node and links it to its
/// parent, building a tree that mirrors the ECS hierarchy.
fn build_taffy_tree(
    taffy: &mut TaffyTree<()>,
    parent_id: NodeId,
    children: &Children,
    ui_nodes: &Query<(Entity, &UINode, Option<&Children>)>,
    entity_to_taffy: &mut HashMap<Entity, NodeId>,
) {
    for child in children {
        let Some((_, child_node, grand_children)) = ui_nodes.get_entity(*child) else {
            continue;
        };
        let Ok(child_id) = taffy.new_leaf(child_node.style()) else {
            continue;
        };
        if taffy.add_child(parent_id, child_id).is_err() {
            continue;
        }
        entity_to_taffy.insert(*child, child_id);

        if let Some(grand_children) = grand_children {
            build_taffy_tree(taffy, child_id, grand_children, ui_nodes, entity_to_taffy);
        }
    }
}

/// Recursively writes [`UIComputedNode`] with **absolute** screen coordinates
/// for every node in the subtree rooted at `children`.
///
/// `parent_origin` is the absolute screen position of the parent so we can
/// convert each child's parent-relative `location` to an absolute position.
fn write_absolute_positions(
    taffy: &TaffyTree<()>,
    parent_origin: Vec2,
    parent_z: i32,
    children: &Children,
    ui_nodes: &Query<(Entity, &UINode, Option<&Children>)>,
    entity_to_taffy: &HashMap<Entity, NodeId>,
    cmd: &mut CommandQueue,
) {
    let z = parent_z + 1;
    for child_entity in children {
        let Some(&node_id) = entity_to_taffy.get(child_entity) else {
            continue;
        };
        let Ok(layout) = taffy.layout(node_id) else {
            continue;
        };

        // layout.location is relative to the parent — add the parent's
        // absolute position to obtain the screen-space position.
        let abs_pos = parent_origin + Vec2::new(layout.location.x, layout.location.y);
        let size = Vec2::new(layout.size.width, layout.size.height);

        cmd.insert(
            UIComputedNode {
                location: abs_pos,
                size,
                z_index: z,
            },
            *child_entity,
        );

        let Some((_, _, Some(grand_children))) = ui_nodes.get_entity(*child_entity) else {
            continue;
        };
        write_absolute_positions(
            taffy,
            abs_pos,
            z,
            grand_children,
            ui_nodes,
            entity_to_taffy,
            cmd,
        );
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
                uv: [0.0, 0.0],
            }, // top-left
            UIVertex {
                pos_coords: to_ndc(x, y + h),
                uv: [0.0, 1.0],
            }, // bottom-left
            UIVertex {
                pos_coords: to_ndc(x + w, y + h),
                uv: [1.0, 1.0],
            }, // bottom-right
            UIVertex {
                pos_coords: to_ndc(x + w, y),
                uv: [1.0, 0.0],
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
                let render_entity = *cmd.spawn(render_ui_node).entity();
                cmd.insert(RenderEntity::new(render_entity), computed_node_entity);
            }
        }
    }
}

/// Syncs the computed node size into [`UIMaterial::border_params`] each frame so
/// the border shader knows the pixel dimensions of the quad it's running on.
///
/// This runs in `LateUpdate` after `compute_ui_nodes`.  Writing `border_params`
/// marks the material as changed, which causes `extract_added_ui_materials` to
/// rebuild the bind group with the updated data.
pub(crate) fn sync_border_size(nodes: Query<(&UIComputedNode, &mut UIMaterial)>) {
    for (node, mut material) in nodes.iter() {
        material.border_params = [material.border_width, node.size.x, node.size.y, 0.0];
    }
}

/// Marker component: this UI node displays the output of a camera render target.
/// Attach alongside [`UINode`] to display the output of a camera that renders
/// to a [`Texture`] render target.  The `texture` field must be the same
/// [`AssetHandle<Texture>`] that was passed to [`Camera::render_target`].
#[derive(Component)]
pub struct UIViewport {
    pub texture: AssetHandle<Texture>,
}

/// GPU-side bind group for a [`UIViewport`] node, updated every frame by
/// `extract_viewport_nodes` so it always references the current RTT texture view.
#[derive(Component)]
pub(crate) struct RenderUIViewport {
    pub(crate) bind_group: wgpu::BindGroup,
}

/// Pipeline and bind group layout for rendering [`UIViewport`] nodes.
///
/// Built in [`UIPlugin::finish`] and stored as a resource.
#[derive(Resource)]
pub struct UIViewportPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

/// Creates a fresh [`RenderUIViewport`] bind group every frame by finding the
/// camera whose `render_target` handle matches the viewport's `texture` handle,
/// then borrowing `color_target.view` directly.
///
/// The bind group is recreated each frame so resize is handled automatically:
/// [`sync_camera_render_textures`] replaces `color_target` on resize, and the
/// next frame's bind group references the new allocation.
pub(crate) fn extract_viewport_nodes(
    viewports: Query<(Entity, &UIViewport, Option<&RenderEntity>)>,
    cameras: Query<(&Camera, &RenderEntity)>,
    render_cameras: Query<(&RenderCamera,)>,
    pipeline: Res<UIViewportPipeline>,
    device: Res<RenderDevice>,
    mut cmd: CommandQueue,
) {
    for (entity, viewport, render_entity) in viewports.iter() {
        // Find the render camera whose handle ID matches this viewport's texture.
        // The bind group is created inside the find_map closure while the
        // render_cameras borrow is live; wgpu::BindGroup does not borrow from
        // the view/sampler after creation, so it's safe to return owned.
        let Some(bind_group) = cameras.iter().find_map(|(cam, re)| {
            let RenderTarget::Texture(h) = &cam.render_target else {
                return None;
            };
            if h.id() != viewport.texture.id() {
                return None;
            }
            let (rc,) = render_cameras.get_entity(**re)?;
            let ct = rc.render_target.as_ref()?;
            Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("UIViewport BindGroup"),
                layout: &pipeline.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&ct.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&ct.sampler),
                    },
                ],
            }))
        }) else {
            continue;
        };

        let rv = RenderUIViewport { bind_group };
        match render_entity {
            Some(re) => cmd.insert(rv, **re),
            None => {
                let new_re = *cmd.spawn(rv).entity();
                cmd.insert(RenderEntity::new(new_re), entity);
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
                let render_entity = *cmd.spawn(render_ui_material).entity();
                cmd.insert(RenderEntity::new(render_entity), computed_node_entity);
            }
        }
    }
}
