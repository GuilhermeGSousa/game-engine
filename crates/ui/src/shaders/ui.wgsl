struct VertexInput {
    @location(0) position: vec2<f32>,
    /// Normalised coords: (0,0) = top-left, (1,1) = bottom-right.
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

/// Background fill colour.
@group(0) @binding(0) var<uniform> color: vec4<f32>;

/// Border outline colour.
@group(0) @binding(1) var<uniform> border_color: vec4<f32>;

/// Border parameters: [border_width_px, node_width_px, node_height_px, unused].
/// Filled by the engine each frame; users set border_width only.
@group(0) @binding(2) var<uniform> border_params: vec4<f32>;

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
    let border_w = border_params.x;
    let node_w = border_params.y;
    let node_h = border_params.z;

    // Only draw a border when border_width > 0 and the node has a known size.
    if border_w > 0.0 && node_w > 0.0 && node_h > 0.0 {
        // Convert normalised UV to pixel coordinates within the node.
        let px = in.uv.x * node_w;
        let py = in.uv.y * node_h;

        if px < border_w || px > node_w - border_w
        || py < border_w || py > node_h - border_w {
            return border_color;
        }
    }

    return color;
}
