use color::LinearRgba;
use ecs::component::Component;
use essential::assets::Asset;
use render::{AsBindGroup, assets::vertex::VertexBufferLayout};

use crate::vertex::UIVertex;

/// Material for UI elements.
///
/// # Border rendering
///
/// Set `border_width` (pixels) and `border_color` to draw a solid rectangular
/// outline.  The engine automatically syncs the node's computed pixel size into
/// `border_params` each frame, so you only need to supply `border_width`.
///
/// # Example
/// ```rust,ignore
/// UIMaterial {
///     color: LinearRgba::new(0.15, 0.15, 0.15, 1.0),
///     border_color: LinearRgba::new(0.4, 0.4, 0.4, 1.0),
///     border_width: 1.0,
///     ..UIMaterial::flat(LinearRgba::new(0.15, 0.15, 0.15, 1.0))
/// }
/// ```
#[derive(Component, Asset, AsBindGroup)]
#[material(
    vertex_shader = include_str!("shaders/ui.wgsl"),
    fragment_shader = include_str!("shaders/ui.wgsl"),
    camera = false,
    depth_stencil = "none",
    vertex_layouts = vec![UIVertex::describe()],
)]
pub struct UIMaterial {
    /// Background fill colour (RGBA, values in `[0.0, 1.0]`).
    #[uniform(0)]
    pub color: LinearRgba,

    /// Border outline colour (RGBA).  Only visible when `border_width > 0`.
    #[uniform(1)]
    pub border_color: LinearRgba,

    /// GPU-side border parameters — **do not set manually**.
    ///
    /// Layout: `[border_width_px, node_width_px, node_height_px, 0.0]`.
    /// The `sync_border_size` system fills in the node dimensions each frame;
    /// `border_width_px` is copied from the user-facing `border_width` field.
    #[uniform(2)]
    pub border_params: [f32; 4],

    /// Border width in logical pixels.  Set this; the engine manages
    /// `border_params` automatically.
    pub border_width: f32,
}

impl UIMaterial {
    /// A plain filled rectangle with no border.
    pub fn flat(color: LinearRgba) -> Self {
        Self {
            color,
            border_color: LinearRgba::TRANSPARENT,
            border_width: 0.0,
            border_params: [0.0; 4],
        }
    }

    /// A filled rectangle with a solid-colour border.
    pub fn with_border(color: LinearRgba, border_color: LinearRgba, border_width: f32) -> Self {
        Self {
            color,
            border_color,
            border_width,
            border_params: [border_width, 0.0, 0.0, 0.0],
        }
    }
}

impl Default for UIMaterial {
    fn default() -> Self {
        Self::flat(LinearRgba::WHITE)
    }
}
