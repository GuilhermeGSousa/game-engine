struct CameraUniform {
    view_pos: vec3<f32>,
    view_proj: mat4x4<f32>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) bone_indices: vec4<u32>,
    @location(6) bone_weights: vec4<f32>,
};

struct TransformInput {
    // Full transform
    @location(7) model_matrix_0: vec4<f32>,
    @location(8) model_matrix_1: vec4<f32>,
    @location(9) model_matrix_2: vec4<f32>,
    @location(10) model_matrix_3: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
};

// group(1) = camera uniform
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(
    model: VertexInput,
    instance: TransformInput) -> VertexOutput {

    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    let world_position = model_matrix * vec4<f32>(model.position, 1.0);

    var output: VertexOutput;
    output.position = camera.view_proj * world_position;
    return output;
}

@group(0) @binding(0)
var<uniform> gizmo_color: vec4f;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    return gizmo_color;
}
