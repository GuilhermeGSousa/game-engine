struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main() -> VertexOutput
{
    var out: VertexOutput;
    return out;
}

@fragment
fn fs_main() {
}