struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

struct VertexInput {
    @location(0) position: vec2<f32>,
};

/// Material colour uniform (RGBA) at group 0, matching UIMaterial's bind group.
@group(0) @binding(0) var<uniform> color: vec4<f32>;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput
{
    // Vertices are pre-transformed to NDC by extract_added_ui_nodes.
    var out: VertexOutput;
    out.position = vec4<f32>(model.position, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32>
{
    return color;
}