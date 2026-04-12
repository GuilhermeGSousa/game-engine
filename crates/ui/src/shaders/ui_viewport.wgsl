struct VertexInput {
    @location(0) position: vec2<f32>,
    /// Normalised coords: (0,0) = top-left, (1,1) = bottom-right.
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    // Vertices are pre-transformed to NDC by extract_added_ui_nodes.
    var out: VertexOutput;
    out.position = vec4<f32>(model.position, 0.0, 1.0);
    out.uv = model.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(scene_texture, scene_sampler, in.uv);
}
