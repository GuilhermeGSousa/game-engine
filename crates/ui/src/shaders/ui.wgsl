struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

struct VertexInput {
    @location(0) position: vec2<f32>,
    // @location(1) tex_coords: vec2<f32>,
};

struct UITransformInput {
    @location(1) matrix: vec4<f32>,
    @location(2) translation: vec2<f32>
}

@group(0) @binding(0) var<uniform> proj_view: mat4x4<f32>

@vertex
fn vs_main(
    model: VertexInput,
    transform: UITransformInput
) -> VertexOutput
{
    var out: VertexOutput;
    out.position = proj_view *  vec4<f32>(model.position, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32>
{
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}