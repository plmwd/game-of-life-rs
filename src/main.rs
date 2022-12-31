#![allow(dead_code)]

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{collections::HashSet, io, thread, time::Duration};
use tui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders},
    Terminal,
};

// Steps:
// 1. Iterate through each alive cell and perform GoL rules
// 2. Render board in frame w/ frame counter
//
// Future:
// 1. Game controls w/ toggle-able UI
//      - pause, play, iteration speed, start from 0
// 2. Zoom/pan though map w/ info bar
//      - enter coords
// 3. Load/export patterns
// 4. Load/export pre-set boards
// 5. Population graphs
// 6. Change grid color based:
//      - visited cells
//      - birth/death of cells

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
struct Position {
    x: i64,
    y: i64,
}

#[derive(Debug, PartialEq, Eq)]
enum Cell {
    Dead(Position),
    Alive(Position),
}

// Need to:
// 1. Iterate (immutable and mutable) over each alive cell
// 2. Iterate over coord slice
// 3. Query (x, y) cell
// 4. Add/remove (x, y) cell
#[derive(Debug, Default, PartialEq, Clone)]
struct Board {
    board: HashSet<Position>,
}

#[derive(PartialEq, Eq)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
}

struct Neighbors<'a> {
    board: &'a Board,
    pos: Position,
    which: u8,
}

impl From<(i64, i64)> for Position {
    fn from(value: (i64, i64)) -> Self {
        Position {
            x: value.0,
            y: value.1,
        }
    }
}

impl<'a> Neighbors<'a> {
    fn new(board: &'a Board, pos: Position) -> Neighbors<'a> {
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
        let Position { x, y } = self.pos;
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
    fn query(&self, pos: &Position) -> Cell {
        match self.board.contains(pos) {
            true => Cell::Alive(*pos),
            false => Cell::Dead(*pos),
        }
    }

    fn neighbors(&self, pos: &Position) -> Neighbors<'_> {
        Neighbors::new(self, *pos)
    }

    fn birth_cell(&mut self, pos: &Position) {
        self.board.insert(*pos);
    }

    fn kill_cell(&mut self, pos: &Position) {
        self.board.remove(pos);
    }

    fn iter(&self) -> impl Iterator<Item = &Position> + '_ {
        self.board.iter()
    }
}

impl<const N: usize> From<[Position; N]> for Board {
    fn from(value: [Position; N]) -> Self {
        Board {
            board: HashSet::from(value),
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
struct GameOfLife {
    board: Board,
    killed_cells: HashSet<Position>,
    birthed_cells: HashSet<Position>,
    generation: u32,
}

impl<const N: usize> From<[Position; N]> for GameOfLife {
    fn from(value: [Position; N]) -> Self {
        GameOfLife {
            board: Board::from(value),
            ..GameOfLife::default()
        }
    }
}

impl GameOfLife {
    fn step(&mut self) {
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
            println!("{num_alive}");
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

// Contains game, user config, UI state, handles events
struct App {}

fn main() -> Result<(), io::Error> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|f| {
        let size = f.size();
        let block = Block::default().title("Block").borders(Borders::ALL);
        f.render_widget(block, size);
    })?;

    thread::sleep(Duration::from_millis(5000));

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

// TODO: impl FromStr for Board
#[cfg(test)]
mod test {
    use crate::*;

    // #[test]
    // fn neighbors_iter() {
    //     let board = Board::from([
    //         (0i64, 0i64).into(),
    //         (0, 1).into(),
    //         (1, 0).into(),
    //         (-1, 0).into(),
    //     ]);
    //
    //     for n in board.neighbors(&(0i64, 0i64).into()) {
    //         println!("{n:?}");
    //     }
    // }

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
    fn idle_board() {
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
}
