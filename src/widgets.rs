use tui::{
    style::{Color, Style},
    widgets::Widget,
};

use crate::{board::Board, point::Point};

pub struct BoardWidget<'b> {
    board: &'b Board,
    origin: Point,
    // TODO: zoom
}

impl<'b> BoardWidget<'b> {
    pub fn new(board: &'b Board) -> Self {
        BoardWidget {
            board,
            origin: Default::default(),
        }
    }

    pub fn pan_to(mut self, origin: Point) -> Self {
        self.origin = origin;
        self
    }
}

impl<'b> Widget for BoardWidget<'b> {
    fn render(self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        buf.set_style(area, Style::default().bg(Color::LightBlue));

        for x in area.x..area.x + area.width {
            for y in area.y..area.y + area.height {
                buf.get_mut(x, y).set_symbol("·").set_fg(Color::Black);
            }
        }
        for (_point, dx, dy) in self.board.window(
            self.origin - Point::new(area.width as i64 / 2, area.height as i64 / 2),
            area.width,
            area.height,
        ) {
            buf.get_mut(area.x + dx, area.y + dy)
                .set_symbol(tui::symbols::bar::FULL);
        }
    }
}
