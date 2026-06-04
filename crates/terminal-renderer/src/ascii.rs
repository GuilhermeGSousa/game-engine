const CHARS: &[u8] = b" .:-=+*#%@";

pub fn luma_to_char(luma: u8) -> char {
    let idx = (luma as usize * (CHARS.len() - 1)) / 255;
    CHARS[idx] as char
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

pub const fn align_up(num: u32, align: u32) -> u32 {
    ((num) + ((align) - 1)) & !((align) - 1)
}

pub fn padded_bytes_per_row(width: u32, format: wgpu::TextureFormat) -> u32 {
    let block_copy_size = format
        .block_copy_size(Some(wgpu::TextureAspect::All))
        .unwrap();

    align_up(width * block_copy_size, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_luma_to_char_black() {
        assert_eq!(luma_to_char(0), ' ');
    }

    #[test]
    fn test_luma_to_char_white() {
        assert_eq!(luma_to_char(255), '@');
    }

    #[test]
    fn test_luma_midrange() {
        let c = luma_to_char(128);
        assert_ne!(c, ' ');
        assert_ne!(c, '@');
    }

    #[test]
    fn test_padded_bytes_per_row_alignment() {
        for width in [1u32, 4, 16, 64, 80, 160, 256, 320] {
            let pbr = padded_bytes_per_row(width, wgpu::TextureFormat::Rgba8Unorm);
            assert_eq!(
                pbr % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT,
                0,
                "width={width} pbr={pbr}"
            );
            assert!(
                pbr >= width * 4,
                "padded must be >= unpadded for width={width}"
            );
        }
    }
}
