use std::io::Stdout;
use tui::{backend::CrosstermBackend, layout::Rect};

pub type Frame<'a> = tui::Frame<'a, CrosstermBackend<Stdout>>;
pub type Terminal = tui::Terminal<CrosstermBackend<Stdout>>;

pub fn within(rect: &Rect, x: u16, y: u16) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}
