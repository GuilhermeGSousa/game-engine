use super::terminal_device::Color;
use super::colors::RgbToAnsiMapper;
use ecs::resource::Resource;

#[derive(Resource)]
pub struct AsciiConverter {
    // Characters mapped by brightness (0-9, where 0 is darkest)
    // Using both ASCII intensity and Unicode box-drawing
    chars: [char; 10],
    color_mapper: RgbToAnsiMapper,
}

impl AsciiConverter {
    pub fn new() -> Self {
        // Brightness gradient from dark (index 0) to bright (index 9)
        let chars = [' ', '.', ':', ';', '+', '=', '*', '#', '▓', '█'];

        Self {
            chars,
            color_mapper: RgbToAnsiMapper::new(),
        }
    }

    pub fn with_chars(chars: [char; 10]) -> Self {
        Self {
            chars,
            color_mapper: RgbToAnsiMapper::new(),
        }
    }

    /// Calculate luminance from RGB using standard formula
    pub fn rgb_to_brightness(r: u8, g: u8, b: u8) -> u8 {
        // Standard luminance formula
        let luminance = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) as u32;
        ((luminance * 10) / 256) as u8
    }

    /// Convert a pixel to ASCII character and color
    pub fn pixel_to_char_color(&self, color: Color) -> (char, u32) {
        let brightness = Self::rgb_to_brightness(color.r, color.g, color.b);
        let brightness_idx = (brightness.min(9)) as usize;
        let ch = self.chars[brightness_idx];
        let ansi_color = self.color_mapper.rgb_to_ansi256(color.r, color.g, color.b);

        (ch, ansi_color)
    }

    /// Convert pixel buffer to screen grid
    /// Assumes pixels are in RGBA format, 4 bytes per pixel
    pub fn pixels_to_screen(
        &self,
        pixel_data: &[u8],
        width: usize,
        height: usize,
    ) -> Vec<Vec<(char, u32)>> {
        let bytes_per_pixel = 4; // RGBA

        let mut screen = vec![vec![(' ', 0u32); width]; height];

        for y in 0..height {
            for x in 0..width {
                let pixel_idx = (y * width + x) * bytes_per_pixel;
                if pixel_idx + 3 < pixel_data.len() {
                    let r = pixel_data[pixel_idx];
                    let g = pixel_data[pixel_idx + 1];
                    let b = pixel_data[pixel_idx + 2];
                    // Alpha is pixel_data[pixel_idx + 3], but we ignore it for now

                    let color = Color { r, g, b };
                    screen[y][x] = self.pixel_to_char_color(color);
                }
            }
        }

        screen
    }
}

impl Default for AsciiConverter {
    fn default() -> Self {
        Self::new()
    }
}
