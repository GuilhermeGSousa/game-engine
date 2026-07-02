use ecs::resource::Resource;
use std::sync::Arc;

#[derive(Resource)]
pub struct RenderContext {
    pub(crate) surface: Option<Arc<wgpu::Surface<'static>>>,
    pub surface_config: wgpu::SurfaceConfiguration,
}

/// Tracks how many `draw_indexed` calls the 3D mesh render passes issued this
/// frame. Reset at the start of each frame in `clear_cameras` and incremented at
/// each `draw_indexed` call site in `material_renderpass`. Exists as ongoing,
/// low-overhead visibility into draw-call counts (e.g. to confirm mesh
/// instancing is actually collapsing per-entity draws into per-batch draws).
#[derive(Resource, Default)]
pub struct DrawCallStats {
    pub draw_calls: u32,
}
