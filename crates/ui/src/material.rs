use ecs::component::Component;
use render::AsBindGroup;

/// Material for UI elements.
///
/// Carries a solid `color` (RGBA, each channel in the range `0.0 – 1.0`) that
/// is uploaded as a uniform buffer and bound at `@group(1) @binding(0)` in the
/// UI shader.
///
/// The bind-group layout and bind-group creation are fully macro-generated via
/// `#[derive(AsBindGroup)]`, so the layout definition always stays in sync with
/// the struct definition.
#[derive(Component, AsBindGroup)]
pub struct UIMaterial {
    /// RGBA colour as `[r, g, b, a]` with values in `[0.0, 1.0]`.
    #[uniform(0)]
    pub color: [f32; 4],
}
