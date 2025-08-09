struct Light {
    color: vec4<f32>,
    position: vec3<f32>,
};

const MAX_LIGHT_COUNT : i32 = 256;

struct Lights {
    lights: array<Light, MAX_LIGHT_COUNT>,
    light_count: i32,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
};


@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
}

struct TransformInput {
    // Full transform
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,

    // Rotation Matrix
    @location(9) rotation_matrix_0: vec3<f32>,
    @location(10) rotation_matrix_1: vec3<f32>,
    @location(11) rotation_matrix_2: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    // TODO: Do this in view space instead!
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: TransformInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    let rotation_matrix = mat3x3<f32>(
        instance.rotation_matrix_0,
        instance.rotation_matrix_1,
        instance.rotation_matrix_2,
    );
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.world_normal = rotation_matrix * model.normal;
    var world_position: vec4<f32> = model_matrix * vec4<f32>(model.position, 1.0);
    out.world_position = world_position.xyz;
    out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@group(2) @binding(0)
var<uniform> lights: Lights;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let ambient_strength = 0.1;

    let light_dir = normalize(lights.lights[0].position - in.world_position);

    let diffuse_strength = max(dot(in.world_normal, light_dir), 0.0);
    let diffuse_color = lights.lights[0].color * diffuse_strength;

    let ambient_color = lights.lights[0].color * ambient_strength;

    let result = (ambient_color.xyz + diffuse_color.xyz) * object_color.xyz;

    return vec4<f32>(result, object_color.a);
}