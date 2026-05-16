/// Maps RGB colors to ANSI 256-color codes
pub struct RgbToAnsiMapper;

impl RgbToAnsiMapper {
    pub fn new() -> Self {
        Self
    }

    /// Convert RGB to ANSI 256-color code (palette index 0-255)
    /// Codes 0-15: basic colors
    /// Codes 16-231: 216-color cube
    /// Codes 232-255: grayscale
    pub fn rgb_to_ansi256(&self, r: u8, g: u8, b: u8) -> u32 {
        // Check if it's grayscale (R ≈ G ≈ B)
        if (r as i32 - g as i32).abs() < 20 && (g as i32 - b as i32).abs() < 20 {
            // Use grayscale range (232-255)
            let gray_level = ((r as u32 + g as u32 + b as u32) / 3) as u8;
            if gray_level < 8 {
                return 16; // Dark gray from color cube
            }
            if gray_level > 247 {
                return 231; // Bright white from color cube
            }
            // 24 levels of grayscale (232-255)
            return 232 + ((gray_level as u32 - 8) / 10).min(23);
        }

        // Use 216-color cube (16-231)
        // Divide each channel into 6 levels (0-5)
        let r_index = (r as u32 * 6) / 256;
        let g_index = (g as u32 * 6) / 256;
        let b_index = (b as u32 * 6) / 256;

        16 + 36 * r_index + 6 * g_index + b_index
    }

    /// Convert to ANSI escape sequence for setting foreground color
    pub fn color_code_to_ansi_sequence(color_code: u32) -> String {
        format!("\x1b[38;5;{}m", color_code)
    }

    /// Reset ANSI color to default
    pub fn reset_color_sequence() -> &'static str {
        "\x1b[0m"
    }
}

impl Default for RgbToAnsiMapper {
    fn default() -> Self {
        Self::new()
    }
}
