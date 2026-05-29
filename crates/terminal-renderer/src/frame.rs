use ecs::Resource;
use ratatui::{style::Style, widgets::Widget};


#[derive(Resource, Default)]
pub struct TerminalFrames
{
    data: Vec<TerminalFrame>
}

impl TerminalFrames {
    pub fn push_data(&mut self, data: String)
    {
        self.data.push(TerminalFrame { content: data });
    }

    pub fn pop_data(&mut self) -> Option<TerminalFrame>
    {
        self.data.pop()
    }
}

pub struct TerminalFrame
{
    content: String,
}

impl Widget for TerminalFrame {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized {
            todo!()
        }
}