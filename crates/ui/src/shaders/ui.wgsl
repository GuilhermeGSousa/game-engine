struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

struct VertexInput {
    @location(0) position: vec2<f32>,
    // @location(1) tex_coords: vec2<f32>,
};

@group(0) @binding(0) var<uniform> proj_view: mat4x4<f32>;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput
{
    var out: VertexOutput;
    out.position = proj_view *  vec4<f32>(model.position, 0.0, 1.0);
    return out;
}

@group(1) @binding(0)
var<uniform> color: vec4<f32>;

@fragment
fn fs_main() -> @location(0) vec4<f32>
{
    return color;
}