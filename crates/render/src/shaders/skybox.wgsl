struct CameraUniform {
    view_pos: vec3<f32>,
    view_proj: mat4x4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) tex_coords: vec3f,
};

@group(0) @binding(0) 
var<uniform> camera: CameraUniform;

@group(1) @binding(0) 
var skybox_texture: texture_cube<f32>;
@group(1) @binding(1)
var skybox_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    // // Use position as direction, discard translation from view matrix
    // output.position = (view_proj * vec4f(position, 1.0)).xyww;
    // output.tex_coords = position;
    return output;
}

@group(1) @binding(0) var cubemap: texture_cube<f32>;
@group(1) @binding(1) var cubemap_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    return textureSample(skybox_texture, skybox_sampler, input.tex_coords);
}