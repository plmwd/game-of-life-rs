use std::io::Stdout;
use tui::backend::CrosstermBackend;

pub type Frame<'a> = tui::Frame<'a, CrosstermBackend<Stdout>>;
pub type Terminal = tui::Terminal<CrosstermBackend<Stdout>>;
