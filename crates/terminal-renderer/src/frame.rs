use ecs::Resource;
use ratatui::{
    style::Style,
    widgets::{Block, Borders, Widget},
};

use crate::ascii::pixels_to_ascii_into;

#[derive(Resource, Default)]
pub struct TerminalFrame {
    buf: Option<String>,
}

impl TerminalFrame {
    pub fn write(&mut self, data: &[u8], width: u32, height: u32, padded_bpr: u32) {
        let mut buf = self.buf.take().unwrap_or_default();
        pixels_to_ascii_into(data, width, height, padded_bpr, &mut buf);
        self.buf = Some(buf);
    }

    pub fn current_frame(&self) -> Option<CurrentFrame<'_>> {
        self.buf.as_deref().map(CurrentFrame)
    }
}

pub struct CurrentFrame<'a>(&'a str);

impl Widget for CurrentFrame<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        block.render(area, buf);

        for (row, line) in self.0.lines().enumerate() {
            let y = inner.y + row as u16;
            if y >= inner.bottom() {
                break;
            }
            buf.set_stringn(inner.x, y, line, inner.width as usize, Style::default());
        }
    }
}
