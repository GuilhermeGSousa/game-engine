const TOON_LEVELS = 3.0; // Number of color bands

const MAX_LIGHT_COUNT : i32 = 256;

const HAS_DIFFUSE_TEXTURE = 1u << 0u;
const HAS_NORMAL_TEXTURE = 1u << 1u;

struct MaterialFlags {
    flags: u32,
}

struct Light {
    position: vec3<f32>,
    // color: vec4<f32>,
    // intensity: f32,
    // direction: vec3<f32>,
    // light_type: u32,
};

struct Lights {
    lights: array<Light, MAX_LIGHT_COUNT>,
    light_count: i32,
};

struct CameraUniform {
    view_pos: vec3<f32>,
    view_proj: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
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
    @location(0) @interpolate(perspective) world_position: vec3<f32>,
    @location(1) @interpolate(perspective) world_normal: vec3<f32>,
    @location(2) @interpolate(perspective) world_tangent: vec3<f32>,
    @location(3) @interpolate(perspective) world_bitangent: vec3<f32>,
    @location(4) tex_coords: vec2<f32>,
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

    let world_position = model_matrix * vec4<f32>(model.position, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.tex_coords = model.tex_coords;
    out.world_position = world_position.xyz;
    out.world_normal = normalize(rotation_matrix * model.normal);
    out.world_tangent = normalize(rotation_matrix * model.tangent);
    out.world_bitangent = normalize(rotation_matrix * model.bitangent);
    return out;
}

// Fragment shader
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;
@group(0) @binding(2)
var t_normal: texture_2d<f32>;
@group(0) @binding(3)
var s_normal: sampler;
@group(0) @binding(4)
var<uniform> material_flags: MaterialFlags;

@group(2) @binding(0)
var<uniform> lights: Lights;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return phong_fs(in);
}

fn phong_fs(in: VertexOutput) -> vec4<f32> {
    let object_color: vec4<f32> = sample_diffuse(in.tex_coords);
    let object_normal: vec4<f32> = sample_normal(in.tex_coords);

     // Normal mapping calculations
    let TBN = mat3x3<f32>(in.world_tangent, in.world_bitangent, in.world_normal);
    let mapped_normal = normalize(TBN * normalize(object_normal.xyz * 2.0 - 1.0));

    let view_dir = normalize(camera.view_pos - in.world_position);

    var total_light = object_color * 0.0;

    for (var i: i32 = 0; i < min(lights.light_count, MAX_LIGHT_COUNT); i = i + 1) {

        let light = lights.lights[i];

        let light_delta = light.position.xyz - in.world_position;
        let distance = length(light_delta);
        let light_dir = normalize(light_delta);

        // Simple Lambertian diffuse
        let NdotL = max(dot(mapped_normal, light_dir), 0.0);
        let diffuse = object_color * NdotL;

        // Simple Blinn-Phong specular
        let halfway_dir = normalize(light_dir + view_dir);
        let specular = pow(max(dot(mapped_normal, halfway_dir), 0.0), 32.0);

        total_light += (diffuse);
        // total_light += vec4<f32>(light.position, 1.0);
    }

    return vec4<f32>(total_light.rgb, object_color.a);
}


fn sample_diffuse(tex_coords: vec2<f32>) -> vec4<f32> {
    if (material_flags.flags & HAS_DIFFUSE_TEXTURE) != 0u {
        return textureSample(t_diffuse, s_diffuse, tex_coords);
    } else {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0); // Default Magenta
    }
}

fn sample_normal(tex_coords: vec2<f32>) -> vec4<f32> {
    if (material_flags.flags & HAS_NORMAL_TEXTURE) != 0u {
        return textureSample(t_normal, s_normal, tex_coords);
    } else {
        return vec4<f32>(0.5, 0.5, 1.0, 1.0); // Default flat normal
    }
}