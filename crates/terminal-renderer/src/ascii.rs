const CHARS: &[u8] = b" .:-=+*#%@";
const COPY_ROW_ALIGNMENT: u32 = 256;

pub fn luma_to_char(luma: u8) -> char {
    let idx = (luma as usize * (CHARS.len() - 1)) / 255;
    CHARS[idx] as char
}

pub fn padded_bytes_per_row(width: u32) -> u32 {
    let unpadded = width * 4;
    (unpadded + COPY_ROW_ALIGNMENT - 1) / COPY_ROW_ALIGNMENT * COPY_ROW_ALIGNMENT
}

pub fn pixels_to_ascii(data: &[u8], width: u32, height: u32, padded_bpr: u32) -> String {
    let mut out = String::with_capacity((width as usize + 1) * height as usize);
    for row in 0..height {
        for col in 0..width {
            let offset = (row * padded_bpr + col * 4) as usize;
            let r = data[offset] as f32;
            let g = data[offset + 1] as f32;
            let b = data[offset + 2] as f32;
            let luma = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
            out.push(luma_to_char(luma));
        }
        out.push('\r');
        out.push('\n');
    }
    out
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
    fn test_pixels_all_black() {
        let width = 4u32;
        let height = 2u32;
        let pbr = padded_bytes_per_row(width);
        let data = vec![0u8; (pbr * height) as usize];
        let ascii = pixels_to_ascii(&data, width, height, pbr);
        assert!(ascii.chars().filter(|&c| c != '\n').all(|c| c == ' '));
    }

    #[test]
    fn test_pixels_all_white() {
        let width = 4u32;
        let height = 2u32;
        let pbr = padded_bytes_per_row(width);
        let data = vec![255u8; (pbr * height) as usize];
        let ascii = pixels_to_ascii(&data, width, height, pbr);
        assert!(ascii.chars().filter(|&c| c != '\n').all(|c| c == '@'));
    }

    #[test]
    fn test_padded_bytes_per_row_alignment() {
        for width in [1u32, 4, 16, 64, 80, 160, 256, 320] {
            let pbr = padded_bytes_per_row(width);
            assert_eq!(pbr % COPY_ROW_ALIGNMENT, 0, "width={width} pbr={pbr}");
            assert!(
                pbr >= width * 4,
                "padded must be >= unpadded for width={width}"
            );
        }
    }
}
