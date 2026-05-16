use super::terminal_device::Color;

#[derive(Clone)]
pub struct ScreenBuffer {
    pub chars: Vec<Vec<char>>,
    pub colors: Vec<Vec<Color>>,
    width: usize,
    height: usize,
}

impl ScreenBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            chars: vec![vec![' '; width]; height],
            colors: vec![vec![Color::white(); width]; height],
            width,
            height,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, ch: char, color: Color) {
        if x < self.width && y < self.height {
            self.chars[y][x] = ch;
            self.colors[y][x] = color;
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Option<(char, Color)> {
        if x < self.width && y < self.height {
            Some((self.chars[y][x], self.colors[y][x]))
        } else {
            None
        }
    }

    pub fn clear(&mut self, bg_color: Color) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.chars[y][x] = ' ';
                self.colors[y][x] = bg_color;
            }
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            self.chars = vec![vec![' '; width]; height];
            self.colors = vec![vec![Color::white(); width]; height];
        }
    }
}
