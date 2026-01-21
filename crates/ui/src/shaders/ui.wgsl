struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) vert_pos: vec3<f32>,
}

struct VertexInput {
    @location(0) position: vec2<f32>,
};

@vertex
fn vs_main(model: VertexInput,) -> VertexOutput
{
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
    out.vert_pos = out.clip_position.xyz;
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32>
{
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}