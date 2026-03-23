// ─── Unlit material shader ────────────────────────────────────────────────────
//
// Renders geometry with a flat tint colour — no lighting, no textures, no
// skeletal animation.  Only two bind groups are needed:
//
//   @group(0) @binding(0)  →  TintUniform  { color: vec4<f32>, _padding: vec4<f32> }
//   @group(1) @binding(0)  →  CameraUniform { view_pos: vec3<f32>, view_proj: mat4x4<f32> }
//
// Because UnlitMaterial sets `needs_lighting = false` and `needs_skeleton = false`,
// the engine's `MaterialPlugin` only includes groups 0 and 1 in the pipeline
// layout.  There is no need to declare @group(2) or @group(3) here.

struct TintUniform {
    color: vec4<f32>,
    _padding: vec4<f32>,
}

struct CameraUniform {
    view_pos: vec3<f32>,
    view_proj: mat4x4<f32>,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    // locations 5 (bone_indices) and 6 (bone_weights) are part of the shared
    // vertex format but are not declared here because this shader does not
    // perform skeletal animation.  The GPU simply ignores the unused attributes.
}

struct TransformInput {
    @location(7)  model_matrix_0: vec4<f32>,
    @location(8)  model_matrix_1: vec4<f32>,
    @location(9)  model_matrix_2: vec4<f32>,
    @location(10) model_matrix_3: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

// ─── Bindings ──────────────────────────────────────────────────────────────────

@group(0) @binding(0) var<uniform> tint: TintUniform;
@group(1) @binding(0) var<uniform> camera: CameraUniform;

// ─── Vertex stage ──────────────────────────────────────────────────────────────

@vertex
fn vs_main(model: VertexInput, instance: TransformInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var out: VertexOutput;
    out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
    return out;
}

// ─── Fragment stage ────────────────────────────────────────────────────────────

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return tint.color;
}
