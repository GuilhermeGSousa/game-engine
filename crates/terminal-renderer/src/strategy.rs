use render::{components::camera::RenderCamera, render_asset::render_texture::RenderTexture};

const CHARS: &[u8] = b" .:-=+*#%@";

pub fn luma_to_char(luma: u8) -> char {
    let idx = (luma as usize * (CHARS.len() - 1)) / 255;
    CHARS[idx] as char
}

#[derive(Clone, Copy, Debug, Default)]
pub enum TerminalRenderStrategy {
    #[default]
    Luminance,
    Depth,
}

impl TerminalRenderStrategy {
    pub fn readback_format(&self) -> wgpu::TextureFormat {
        match self {
            TerminalRenderStrategy::Luminance => wgpu::TextureFormat::Rgba8UnormSrgb,
            TerminalRenderStrategy::Depth => wgpu::TextureFormat::Depth32Float,
        }
    }

    pub fn select_render_texture<'a>(&self, camera: &'a RenderCamera) -> &'a RenderTexture {
        match self {
            TerminalRenderStrategy::Luminance => camera
                .render_target
                .as_ref()
                .expect("Render target not found on camera — make sure the camera is configured for offscreen rendering"),
            TerminalRenderStrategy::Depth => camera.depth_texture(),
        }
    }

    pub fn convert_pixels(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        padded_bpr: u32,
        buffer: &mut String,
    ) {
        match self {
            TerminalRenderStrategy::Luminance => {
                pixels_to_ascii_into(data, width, height, padded_bpr, buffer)
            }
            TerminalRenderStrategy::Depth => {
                depth_to_ascii_into(data, width, height, padded_bpr, buffer)
            }
        }
    }
}

pub fn pixels_to_ascii_into(
    data: &[u8],
    width: u32,
    height: u32,
    padded_bpr: u32,
    out: &mut String,
) {
    out.clear();
    let needed = (width as usize + 2) * height as usize;
    if out.capacity() < needed {
        out.reserve(needed - out.capacity());
    }

    for row in 0..height {
        for col in 0..width {
            let offset = (row * padded_bpr + col * 4) as usize;
            let r = data[offset] as f32;
            let g = data[offset + 1] as f32;
            let b = data[offset + 2] as f32;
            let luma = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
            out.push(luma_to_char(luma));
        }
        out.push('\n');
    }
}

pub fn depth_to_ascii_into(
    data: &[u8],
    width: u32,
    height: u32,
    padded_bpr: u32,
    out: &mut String,
) {
    out.clear();
    let needed = (width as usize + 2) * height as usize;
    if out.capacity() < needed {
        out.reserve(needed - out.capacity());
    }

    // Camera defaults: znear = 0.1, zfar = 100.0
    const ZNEAR: f32 = 0.1;
    const ZFAR: f32 = 100.0;
    // Linearized depth beyond this distance renders as background (space)
    const MAX_VISIBLE_DEPTH: f32 = 20.0;

    for row in 0..height {
        for col in 0..width {
            let offset = (row * padded_bpr + col * 4) as usize;
            let bytes: [u8; 4] = data[offset..offset + 4].try_into().unwrap();
            let depth = f32::from_le_bytes(bytes);

            let ch = if depth >= 1.0 {
                ' ' // background — clear depth value
            } else {
                // Linearize: convert non-linear [0,1] to view-space distance
                let linear_z = ZNEAR * ZFAR / (ZFAR - depth * (ZFAR - ZNEAR));
                // Normalize over the visible range; invert so close = bright
                let normalized = (1.0 - (linear_z / MAX_VISIBLE_DEPTH).min(1.0)).max(0.0);
                luma_to_char((normalized * 255.0) as u8)
            };
            out.push(ch);
        }
        out.push('\n');
    }
}
