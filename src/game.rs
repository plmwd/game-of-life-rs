use std::{collections::HashSet, fmt::Display, str::FromStr};

use crate::{
    board::{Board, Cell},
    point::Point,
};

#[derive(Debug)]
pub struct GameError {
    kind: GameErrorKind,
}

impl GameError {
    pub fn new(kind: GameErrorKind) -> Self {
        GameError { kind }
    }
}

#[derive(Debug)]
pub enum GameErrorKind {
    InvalidBoardChar { c: char, line: u16, s: String },
}

impl std::error::Error for GameError {}

impl Display for GameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            GameErrorKind::InvalidBoardChar { c, line, s } => f.write_fmt(format_args!(
                "Invalid char `{}` found on line {} when parsing\n{}\n into Board",
                c, line, s
            )),
        }
    }
}

// Contains board and any game parameters
// Game of Life Rules:
// 1. Any live cell with fewer than two live neighbours dies (referred to as underpopulation)
// 2. Any live cell with more than three live neighbours dies (referred to as overpopulation)
// 3. Any live cell with two or three live neighbours lives, unchanged, to the next generation
// 4. Any dead cell with exactly three live neighbours comes to life
#[derive(Debug, Default)]
pub struct GameOfLife {
    pub board: Board,
    pub killed_cells: HashSet<Point>,
    pub birthed_cells: HashSet<Point>,
    pub generation: u32,
}

impl<const N: usize> From<[Point; N]> for GameOfLife {
    fn from(value: [Point; N]) -> Self {
        GameOfLife {
            board: Board::from(value),
            ..GameOfLife::default()
        }
    }
}

// TODO: use type-state to track game state?
impl GameOfLife {
    pub fn board_from_str(&mut self, s: &str) -> Result<(), GameError> {
        self.board = s.parse()?;
        Ok(())
    }

    pub fn step(&mut self) {
        self.killed_cells.clear();
        self.birthed_cells.clear();

        for pos in self.board.iter() {
            let mut num_alive = 0;
            for cell in self.board.neighbors(pos) {
                match cell {
                    Cell::Dead(pos) => {
                        // Rule 4
                        if let 3 = self
                            .board
                            .neighbors(&pos)
                            .filter(|c| matches!(c, Cell::Alive(_)))
                            .count()
                        {
                            self.birthed_cells.insert(pos);
                        }
                    }
                    Cell::Alive(_) => num_alive += 1,
                }
            }
            match num_alive {
                // Rule 1 & 2
                0 | 1 | 4.. => {
                    self.killed_cells.insert(*pos);
                }
                // Rule 3
                _ => {}
            };
        }

        for pos in &self.killed_cells {
            self.board.kill_cell(pos);
        }

        for pos in &self.birthed_cells {
            self.board.birth_cell(pos);
        }

        self.generation += 1;
    }
}

impl FromStr for GameOfLife {
    type Err = GameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(GameOfLife {
            board: s.parse()?,
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lonely_cell() {
        // No neighbors
        let mut game = GameOfLife::from([(0i64, 0i64).into()]);
        game.step();
        assert_eq!(game.board, Board::default());

        // One neighbor
        let mut game = GameOfLife::from([(0i64, 0i64).into(), (1, 0).into()]);
        game.step();
        assert_eq!(game.board, Board::default());
    }

    #[test]
    fn still_lifes() {
        // Empty
        let mut game = GameOfLife::default();
        game.step();
        assert_eq!(game.board, Board::default());

        // 3 neighbors
        let mut game = GameOfLife::from([
            (0i64, 0i64).into(),
            (1, 0).into(),
            (1, 1).into(),
            (0, 1).into(),
        ]);
        let before = game.board.clone();
        game.step();
        assert_eq!(game.board, before);
    }

    #[test]
    fn sufficated_cell() {
        let mut game = GameOfLife::from([
            (0i64, 0i64).into(),
            (1, 1).into(),
            (-1, 1).into(),
            (-1, -1).into(),
            (1, -1).into(),
        ]);
        game.step();
        assert_eq!(
            game.board,
            Board::from([
                (1i64, 0i64).into(),
                (0, 1).into(),
                (-1, 0).into(),
                (0, -1).into()
            ])
        );
    }

    #[test]
    fn oscillators() {
        todo!()
    }
}
