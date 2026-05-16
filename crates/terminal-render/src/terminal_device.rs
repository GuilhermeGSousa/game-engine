use anyhow::Result;
use crossterm::{
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
    execute,
    style::SetForegroundColor,
};
use std::io::{self, Stdout};
use ecs::resource::Resource;

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
        let width = width as usize;
        let height = height as usize;

        let mut stdout = io::stdout();

        // Enter alternate screen and enable raw mode
        execute!(stdout, EnterAlternateScreen, SetTitle("Terminal Renderer"))?;
        terminal::enable_raw_mode()?;

        Ok(Self {
            width,
            height,
            stdout,
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn clear(&mut self) -> Result<()> {
        execute!(self.stdout, terminal::Clear(terminal::ClearType::All))?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        use std::io::Write;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn update_size(&mut self) -> Result<()> {
        if let Ok((width, height)) = terminal::size() {
            self.width = width as usize;
            self.height = height as usize;
        }
        Ok(())
    }
}

impl Drop for TerminalDevice {
    fn drop(&mut self) {
        let _ = execute!(self.stdout, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}
