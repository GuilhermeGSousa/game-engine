// ─── Unlit material shader ────────────────────────────────────────────────────
//
// Renders geometry with a flat tint colour — no lighting, no textures.
// Bind-group layout (group 0):
//   binding 0 → uniform TintUniform  { color: vec4<f32>, _padding: vec4<f32> }
//
// Groups 1–3 mirror the standard pipeline (camera, lights, bones) so that
// MaterialPlugin can re-use the same pipeline layout.

const MAX_BONE_COUNT: i32 = 128;

struct TintUniform {
    color: vec4<f32>,
    _padding: vec4<f32>,
}

struct CameraUniform {
    view_pos: vec3<f32>,
    view_proj: mat4x4<f32>,
}

struct Skeleton {
    bones: array<mat4x4<f32>, MAX_BONE_COUNT>,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) bone_indices: vec4<u32>,
    @location(6) bone_weights: vec4<f32>,
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
@group(3) @binding(0) var<uniform> bones: Skeleton;

// ─── Vertex stage ──────────────────────────────────────────────────────────────

@vertex
fn vs_main(model: VertexInput, instance: TransformInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    let world_position = model_matrix * vec4<f32>(model.position, 1.0);

    var out: VertexOutput;

    let total_weight = model.bone_weights.x + model.bone_weights.y
                     + model.bone_weights.z + model.bone_weights.w;
    if total_weight > 0.0 {
        var pose = mat4x4<f32>();
        for (var i: i32 = 0; i < 4; i++) {
            pose += bones.bones[model.bone_indices[i]] * model.bone_weights[i];
        }
        out.clip_position = camera.view_proj * pose * world_position;
    } else {
        out.clip_position = camera.view_proj * world_position;
    }

    return out;
}

// ─── Fragment stage ────────────────────────────────────────────────────────────

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return tint.color;
}
