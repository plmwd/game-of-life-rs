use std::{collections::HashSet, str::FromStr};

use crate::{
    game::{GameError, GameErrorKind},
    point::Point,
};

#[derive(Debug, PartialEq, Eq)]
pub enum Cell {
    Dead(Point),
    Alive(Point),
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Board {
    board: HashSet<Point>,
}

pub struct Neighbors<'a> {
    board: &'a Board,
    pos: Point,
    which: u8,
}

impl<'a> Neighbors<'a> {
    pub fn new(board: &'a Board, pos: Point) -> Neighbors<'a> {
        Neighbors {
            board,
            pos,
            which: 0,
        }
    }
}

/// Iterates over neighbors of X in counterclockwise rotation
impl Iterator for Neighbors<'_> {
    type Item = Cell;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = 8 - self.which as usize;
        (len, Some(len))
    }

    fn next(&mut self) -> Option<Self::Item> {
        let Point { x, y } = self.pos;
        let pos = match self.which {
            // right
            0 => (x + 1, y),
            // up-right
            1 => (x + 1, y + 1),
            // up
            2 => (x, y + 1),
            // up-left
            3 => (x - 1, y + 1),
            // left
            4 => (x - 1, y),
            // down-left
            5 => (x - 1, y - 1),
            // down
            6 => (x, y - 1),
            // down-right
            7 => (x + 1, y - 1),
            _ => return None,
        };
        self.which += 1;

        Some(self.board.query(&pos.into()))
    }
}

impl ExactSizeIterator for Neighbors<'_> {}

impl Board {
    pub fn clear(&mut self) {
        self.board.clear();
    }

    pub fn query(&self, pos: &Point) -> Cell {
        match self.board.contains(pos) {
            true => Cell::Alive(*pos),
            false => Cell::Dead(*pos),
        }
    }

    pub fn neighbors(&self, p: &Point) -> Neighbors<'_> {
        Neighbors::new(self, *p)
    }

    pub fn birth_cell(&mut self, p: &Point) {
        self.board.insert(*p);
    }

    pub fn kill_cell(&mut self, p: &Point) {
        self.board.remove(p);
    }

    pub fn toggle_cell(&mut self, p: &Point) {
        if self.board.contains(p) {
            self.kill_cell(p);
        } else {
            self.birth_cell(p);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Point> + '_ {
        self.board.iter()
    }

    pub fn window(
        &self,
        point: Point,
        width: u16,
        height: u16,
    ) -> impl Iterator<Item = (&Point, u16, u16)> + '_ {
        self.board.iter().filter_map(move |p| {
            let dx = p.x - point.x;
            let dy = p.y - point.y;
            if dx >= 0 && dx < width.into() && dy >= 0 && dy < height.into() {
                Some((p, dx as u16, dy as u16))
            } else {
                None
            }
        })
    }
}

impl<const N: usize> From<[Point; N]> for Board {
    fn from(value: [Point; N]) -> Self {
        Board {
            board: HashSet::from(value),
        }
    }
}

/// Builds board from string in the +x +y quadrant where '.' represents a dead cell and 'x'
/// represents an alive one. Any other characters would result in an error.
/// Lines are along the y-axis and chars are along the x-axis. The board can be naturally written
/// meaning the first line is the line with the maximum y value.
///
/// For example,
///  y
///  ^
///  2  ....x...
///  1  ...xxx..
///  0  ....x...
///     01234567 > x
impl FromStr for Board {
    type Err = GameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut board = Board::default();
        for (y, line) in s.lines().rev().enumerate() {
            for (x, char) in line.chars().enumerate() {
                match char {
                    '.' => {} // Do nothing, dead cell
                    'x' => board.birth_cell(&(x as i64, y as i64).into()),
                    c => {
                        return Err(GameError::new(GameErrorKind::InvalidBoardChar {
                            c,
                            s: s.to_owned(),
                            line: y as u16,
                        }))
                    }
                }
            }
        }
        Ok(board)
    }
}
