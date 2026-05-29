use ecs::Resource;

use derive_more::{Deref, DerefMut, From};

#[derive(Resource, Deref, DerefMut, From)]
pub struct Terminal(ratatui::prelude::Terminal<ratatui::prelude::CrosstermBackend<std::io::Stdout>>);

impl Terminal {
    pub fn new(terminal: ratatui::prelude::Terminal<ratatui::prelude::CrosstermBackend<std::io::Stdout>>) -> Terminal
    {
        Terminal(terminal)
    }
}