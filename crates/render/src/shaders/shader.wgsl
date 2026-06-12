const MAX_LIGHT_COUNT : i32 = 128;
const MAX_BONE_COUNT : i32 = 128;

const HAS_BASE_COLOR_TEXTURE = 1u << 0u;
const HAS_NORMAL_TEXTURE = 1u << 1u;
const HAS_METALLIC_ROUGHNESS_TEXTURE = 1u << 2u;
const HAS_EMISSIVE_TEXTURE = 1u << 3u;
const HAS_OCCLUSION_TEXTURE = 1u << 4u;

const POINT_LIGHT = 0u;
const SPOT_LIGHT = 1u;
const DIRECTIONAL_LIGHT = 2u;

const PI = 3.14159265359;
const AMBIENT_INTENSITY = 0.03;
// Roughness below this produces a near-singular specular lobe.
const MIN_ROUGHNESS = 0.045;

struct MaterialUniform {
    base_color_factor: vec4<f32>,
    emissive_factor: vec3<f32>,
    metallic_factor: f32,
    roughness_factor: f32,
    occlusion_strength: f32,
    flags: u32,
    _padding: u32,
}

struct Light {
    position: vec3<f32>,
    intensity: f32,
    color: vec4<f32>,
    direction: vec3<f32>,
    light_type: u32,
    cos_cone_angle: f32,
};

struct Lights {
    lights: array<Light, MAX_LIGHT_COUNT>,
    light_count: i32,
};

struct Skeleton {
    bones: array<mat4x4<f32>, MAX_BONE_COUNT>
};

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

    // Normalizing each column strips non-uniform scale, leaving the pure rotation.
    // A pure rotation matrix is its own inverse-transpose, making this correct for normals.
    let normal_matrix = mat3x3<f32>(
        normalize(instance.model_matrix_0.xyz),
        normalize(instance.model_matrix_1.xyz),
        normalize(instance.model_matrix_2.xyz),
    );

    let world_position = model_matrix * vec4<f32>(model.position, 1.0);

    var out: VertexOutput;

    let total_weight = model.bone_weights.x + model.bone_weights.y + model.bone_weights.z + model.bone_weights.w;
    if total_weight > 0 {
        var pose_transform = mat4x4<f32>();
        for (var i: i32 = 0; i < 4; i = i + 1) {
            pose_transform += bones.bones[model.bone_indices[i]] * model.bone_weights[i];
        }
        out.clip_position = camera.view_proj * pose_transform * world_position;
    } else {
        out.clip_position = camera.view_proj * world_position;
    }

    out.tex_coords = model.tex_coords;
    out.world_position = world_position.xyz;
    out.world_normal = normalize(normal_matrix * model.normal);
    out.world_tangent = normalize(normal_matrix * model.tangent);
    out.world_bitangent = normalize(normal_matrix * model.bitangent);
    return out;
}

// Fragment shader
@group(0) @binding(0)
var t_base_color: texture_2d<f32>;
@group(0) @binding(1)
var s_base_color: sampler;
@group(0) @binding(2)
var t_normal: texture_2d<f32>;
@group(0) @binding(3)
var s_normal: sampler;
@group(0) @binding(4)
var t_metallic_roughness: texture_2d<f32>;
@group(0) @binding(5)
var s_metallic_roughness: sampler;
@group(0) @binding(6)
var t_emissive: texture_2d<f32>;
@group(0) @binding(7)
var s_emissive: sampler;
@group(0) @binding(8)
var t_occlusion: texture_2d<f32>;
@group(0) @binding(9)
var s_occlusion: sampler;
@group(0) @binding(10)
var<uniform> material: MaterialUniform;

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

@group(2) @binding(0)
var<uniform> lights: Lights;

@group(3) @binding(0)
var<uniform> bones: Skeleton;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return pbr_fs(in);
}

fn pbr_fs(in: VertexOutput) -> vec4<f32> {
    let base_color = sample_base_color(in.tex_coords) * material.base_color_factor;
    let metallic_roughness = sample_metallic_roughness(in.tex_coords);
    let metallic = metallic_roughness.b * material.metallic_factor;
    let roughness = clamp(metallic_roughness.g * material.roughness_factor, MIN_ROUGHNESS, 1.0);
    let occlusion = mix(1.0, sample_occlusion(in.tex_coords).r, material.occlusion_strength);
    let emissive = sample_emissive(in.tex_coords).rgb * material.emissive_factor;

    // Normal mapping calculations
    let object_normal = sample_normal(in.tex_coords);
    let TBN = mat3x3<f32>(in.world_tangent, in.world_bitangent, in.world_normal);
    let mapped_normal = normalize(TBN * normalize(object_normal.xyz * 2.0 - 1.0));

    let view_dir = normalize(camera.view_pos - in.world_position);
    let NdotV = max(dot(mapped_normal, view_dir), 1e-4);

    // Dielectrics reflect ~4% at normal incidence; metals reflect base color.
    let f0 = mix(vec3<f32>(0.04), base_color.rgb, metallic);
    let diffuse_color = base_color.rgb * (1.0 - metallic);

    var total_light = vec3<f32>(0.0);

    for (var i: i32 = 0; i < min(lights.light_count, MAX_LIGHT_COUNT); i = i + 1) {

        let light = lights.lights[i];

        let light_delta = light.position.xyz - in.world_position;

        var light_dir = -light.direction;

        let light_type = light.light_type;

        if light_type != DIRECTIONAL_LIGHT {
            light_dir = normalize(light_delta);
        }

        var attenuation = 1.0;
        if light_type != DIRECTIONAL_LIGHT {
            let light_distance_sq = dot(light_delta, light_delta);
            attenuation = 1.0 / max(light_distance_sq, 1e-4);
        }
        if light_type == SPOT_LIGHT {
            let cone_dir = normalize(light.direction.xyz);
            let angle_cos = dot(light_dir, -cone_dir);
            let cone_edge_softness = mix(light.cos_cone_angle, 1.0, 0.2);
            attenuation *= smoothstep(light.cos_cone_angle, cone_edge_softness, angle_cos);
        }

        let radiance = light.color.rgb * light.intensity * attenuation;

        let NdotL = max(dot(mapped_normal, light_dir), 0.0);
        let halfway_dir = normalize(light_dir + view_dir);
        let NdotH = max(dot(mapped_normal, halfway_dir), 0.0);
        let HdotV = max(dot(halfway_dir, view_dir), 0.0);

        // Cook-Torrance specular BRDF
        let D = distribution_ggx(NdotH, roughness);
        let G = geometry_smith(NdotV, NdotL, roughness);
        let F = fresnel_schlick(HdotV, f0);
        let specular = (D * G * F) / max(4.0 * NdotV * NdotL, 1e-4);

        // Energy not reflected as specular diffuses (none for metals)
        let kd = (vec3<f32>(1.0) - F) * (1.0 - metallic);

        total_light += (kd * diffuse_color / PI + specular) * radiance * NdotL;
    }

    let ambient = AMBIENT_INTENSITY * base_color.rgb * occlusion;
    let color = ambient + total_light + emissive;

    // Tone map to LDR; the sRGB surface format applies gamma encoding.
    return vec4<f32>(aces_tonemap(color), base_color.a);
}

// Trowbridge-Reitz GGX normal distribution
fn distribution_ggx(NdotH: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let f = NdotH * NdotH * (a2 - 1.0) + 1.0;
    return a2 / (PI * f * f);
}

// Smith geometry term with Schlick-GGX, k remapped for analytic lights
fn geometry_smith(NdotV: f32, NdotL: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    let ggx_v = NdotV / (NdotV * (1.0 - k) + k);
    let ggx_l = NdotL / (NdotL * (1.0 - k) + k);
    return ggx_v * ggx_l;
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

// Narkowicz ACES filmic approximation
fn aces_tonemap(color: vec3<f32>) -> vec3<f32> {
    let mapped = (color * (2.51 * color + 0.03)) / (color * (2.43 * color + 0.59) + 0.14);
    return clamp(mapped, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn sample_base_color(tex_coords: vec2<f32>) -> vec4<f32> {
    if (material.flags & HAS_BASE_COLOR_TEXTURE) != 0u {
        return textureSample(t_base_color, s_base_color, tex_coords);
    } else {
        return vec4<f32>(1.0);
    }
}

fn sample_normal(tex_coords: vec2<f32>) -> vec4<f32> {
    if (material.flags & HAS_NORMAL_TEXTURE) != 0u {
        return textureSample(t_normal, s_normal, tex_coords);
    } else {
        return vec4<f32>(0.5, 0.5, 1.0, 1.0); // Default flat normal
    }
}

fn sample_metallic_roughness(tex_coords: vec2<f32>) -> vec4<f32> {
    if (material.flags & HAS_METALLIC_ROUGHNESS_TEXTURE) != 0u {
        return textureSample(t_metallic_roughness, s_metallic_roughness, tex_coords);
    } else {
        return vec4<f32>(1.0); // Factors apply unmodified
    }
}

fn sample_emissive(tex_coords: vec2<f32>) -> vec4<f32> {
    if (material.flags & HAS_EMISSIVE_TEXTURE) != 0u {
        return textureSample(t_emissive, s_emissive, tex_coords);
    } else {
        return vec4<f32>(1.0); // Factor applies unmodified
    }
}

fn sample_occlusion(tex_coords: vec2<f32>) -> vec4<f32> {
    if (material.flags & HAS_OCCLUSION_TEXTURE) != 0u {
        return textureSample(t_occlusion, s_occlusion, tex_coords);
    } else {
        return vec4<f32>(1.0); // Unoccluded
    }
}