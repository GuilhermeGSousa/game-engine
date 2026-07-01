const MAX_LIGHT_COUNT: i32 = 128;
const DIRECTIONAL_LIGHT: u32 = 2u;
const SPOT_LIGHT: u32 = 1u;

struct WorldGridUniform {
    line_color: vec4<f32>,
    cell_size: f32,
    coarse_cells: f32,
    fade_start: f32,
    fade_end: f32,
    _padding: vec4<u32>,
};

struct CameraUniform {
    view_pos: vec3<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
};

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

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0)       ndc_pos: vec2<f32>,
};

struct FragOutput {
    @location(0)          color: vec4<f32>,
    @builtin(frag_depth)  depth: f32,
};

@group(0) @binding(0) var<uniform> grid: WorldGridUniform;
@group(1) @binding(0) var<uniform> camera: CameraUniform;
@group(2) @binding(0) var<uniform> lights: Lights;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    const positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let ndc = positions[vi % 3u];
    var out: VertexOutput;
    out.clip_pos = vec4<f32>(ndc, 0.0, 1.0);
    out.ndc_pos = ndc;
    return out;
}

fn grid_line(coord: f32, spacing: f32) -> f32 {
    let d = abs(fract(coord / spacing + 0.5) - 0.5) * spacing;
    let px = fwidth(coord);
    return 1.0 - smoothstep(px, 2.0 * px, d);
}

@fragment
fn fs_main(in: VertexOutput) -> FragOutput {
    let near_h = camera.inv_view_proj * vec4<f32>(in.ndc_pos, 0.0, 1.0);
    let far_h = camera.inv_view_proj * vec4<f32>(in.ndc_pos, 1.0, 1.0);
    let world_near = near_h.xyz / near_h.w;
    let world_far = far_h.xyz / far_h.w;
    let ray_dir = world_far - world_near;

    if abs(ray_dir.y) < 1e-5 { discard; }
    let t = -world_near.y / ray_dir.y;
    if t < 0.0 { discard; }

    let world_pos = world_near + t * ray_dir;

    let fine = max(grid_line(world_pos.x, grid.cell_size),
        grid_line(world_pos.z, grid.cell_size));
    let coarse = max(grid_line(world_pos.x, grid.cell_size * grid.coarse_cells),
        grid_line(world_pos.z, grid.cell_size * grid.coarse_cells));

    let line_alpha = max(fine * 0.4, coarse);
    let brightness = select(1.0, 1.6, coarse > fine);
    let line_color = clamp(grid.line_color.rgb * brightness, vec3<f32>(0.0), vec3<f32>(1.0));

    let grid_normal = vec3<f32>(0.0, 1.0, 0.0);
    var light_accum = vec3<f32>(0.03);

    for (var i: i32 = 0; i < min(lights.light_count, MAX_LIGHT_COUNT); i++) {
        let L = lights.lights[i];
        var light_dir = -L.direction;
        var attenuation = 1.0;

        if L.light_type != DIRECTIONAL_LIGHT {
            let delta = L.position - world_pos;
            light_dir = normalize(delta);
            attenuation = 1.0 / max(dot(delta, delta), 1e-4);
        }
        if L.light_type == SPOT_LIGHT {
            let angle_cos = dot(light_dir, normalize(-L.direction));
            let soft_edge = mix(L.cos_cone_angle, 1.0, 0.2);
            attenuation *= smoothstep(L.cos_cone_angle, soft_edge, angle_cos);
        }

        let NdotL = max(dot(grid_normal, light_dir), 0.0);
        light_accum += L.color.rgb * L.intensity * attenuation * NdotL;
    }

    let lit_color = line_color * clamp(light_accum, vec3<f32>(0.0), vec3<f32>(1.0));
    let dist = length(world_pos.xz - camera.view_pos.xz);
    let dist_alpha = 1.0 - smoothstep(grid.fade_start, grid.fade_end, dist);
    let alpha = line_alpha * grid.line_color.a * dist_alpha;

    if alpha < 0.005 { discard; }

    let clip_hit = camera.view_proj * vec4<f32>(world_pos, 1.0);

    var out: FragOutput;
    out.color = vec4<f32>(lit_color, alpha);
    out.depth = clip_hit.z / clip_hit.w;
    return out;
}
