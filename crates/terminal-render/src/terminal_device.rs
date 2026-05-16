use anyhow::Result;
use crossterm::{cursor, execute, style, terminal};
use ecs::resource::Resource;
use std::io::{self, Stdout, Write};

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn black() -> Self {
        Self { r: 0, g: 0, b: 0 }
    }

    pub fn white() -> Self {
        Self { r: 255, g: 255, b: 255 }
    }

    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Resource)]
pub struct TerminalDevice {
    width: usize,
    height: usize,
    stdout: Stdout,
}

impl TerminalDevice {
    pub fn new() -> Result<Self> {
        let (width, height) = terminal::size()?;
        let mut stdout = io::stdout();

        // Hide cursor to reduce flicker; no alternate screen or raw mode needed
        execute!(stdout, cursor::Hide)?;

        Ok(Self {
            width: width as usize,
            height: height as usize,
            stdout,
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn update_size(&mut self) {
        if let Ok((w, h)) = terminal::size() {
            self.width = w as usize;
            self.height = h as usize;
        }
    }

    /// Write a sampled ASCII frame to the terminal.
    ///
    /// `screen_grid` is addressed as `[tex_y][tex_x]`. The method downsamples
    /// it to `term_width × term_height` characters.
    pub fn render_frame(
        &mut self,
        screen_grid: &[Vec<(char, u32)>],
        tex_width: usize,
        tex_height: usize,
        term_width: usize,
        term_height: usize,
    ) {
        if tex_width == 0 || tex_height == 0 || term_width == 0 || term_height == 0 {
            return;
        }

        let row_step = tex_height as f32 / term_height as f32;
        let col_step = tex_width as f32 / term_width as f32;

        // Go to the top-left corner; do NOT clear — just overwrite, which avoids flicker
        let _ = execute!(self.stdout, cursor::MoveTo(0, 0));

        let mut current_color: u32 = u32::MAX; // sentinel: force first color write

        for ty in 0..term_height {
            let py = ((ty as f32 + 0.5) * row_step) as usize;
            let py = py.min(tex_height - 1);

            for tx in 0..term_width {
                let px = ((tx as f32 + 0.5) * col_step) as usize;
                let px = px.min(tex_width - 1);

                if py < screen_grid.len() && px < screen_grid[py].len() {
                    let (ch, color_code) = screen_grid[py][px];

                    // Only emit a color change escape when the color actually changes
                    if color_code != current_color {
                        let _ = execute!(
                            self.stdout,
                            style::SetForegroundColor(style::Color::AnsiValue(
                                color_code as u8
                            ))
                        );
                        current_color = color_code;
                    }

                    let _ = write!(self.stdout, "{}", ch);
                }
            }

            // Carriage return + newline so columns wrap correctly
            let _ = write!(self.stdout, "\r\n");
        }

        let _ = execute!(self.stdout, style::ResetColor);
        let _ = self.stdout.flush();
    }
}

impl Drop for TerminalDevice {
    fn drop(&mut self) {
        let _ = execute!(self.stdout, cursor::Show, style::ResetColor);
    }
}
