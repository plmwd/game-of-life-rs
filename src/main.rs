#![allow(dead_code)]

use crossterm::{
    event::{poll, read, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{collections::HashSet, fmt::Display, io, str::FromStr, thread, time::Duration};
use tui::{
    backend::CrosstermBackend,
    style::Color,
    symbols,
    widgets::{
        canvas::{Canvas, Line, Map, MapResolution, Rectangle},
        Block, Borders, Widget,
    },
    Terminal,
};

// TODO: impl FromStr for Board

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

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy)]
struct Point {
    x: i64,
    y: i64,
}

impl std::ops::Add for Point {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Point {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::AddAssign for Point {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl std::ops::Sub for Point {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Point {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::SubAssign for Point {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Point {
    fn new(x: i64, y: i64) -> Self {
        Point { x, y }
    }

    fn x(x: i64) -> Self {
        Point {
            x,
            ..Default::default()
        }
    }

    fn y(y: i64) -> Self {
        Point {
            y,
            ..Default::default()
        }
    }

    fn dx(&mut self, x: i64) {
        self.x += x;
    }

    fn dy(&mut self, y: i64) {
        self.y += y;
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Cell {
    Dead(Point),
    Alive(Point),
}

#[derive(Debug)]
struct GameError {
    kind: GameErrorKind,
}

impl GameError {
    fn new(kind: GameErrorKind) -> Self {
        GameError { kind }
    }
}

#[derive(Debug)]
enum GameErrorKind {
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

// Need to:
// 1. Iterate (immutable and mutable) over each alive cell
// 2. Iterate over coord slice
// 3. Query (x, y) cell
// 4. Add/remove (x, y) cell
#[derive(Debug, Default, PartialEq, Clone)]
struct Board {
    board: HashSet<Point>,
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
    pos: Point,
    which: u8,
}

impl From<(i64, i64)> for Point {
    fn from(value: (i64, i64)) -> Self {
        Point {
            x: value.0,
            y: value.1,
        }
    }
}

// TODO: create BoardWidget and implement Widget
// TODO: create methods to create from Board
// TODO: pan/zoom by passing in area: Rect with different offset, width, and height

impl<'a> Neighbors<'a> {
    fn new(board: &'a Board, pos: Point) -> Neighbors<'a> {
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
    fn query(&self, pos: &Point) -> Cell {
        match self.board.contains(pos) {
            true => Cell::Alive(*pos),
            false => Cell::Dead(*pos),
        }
    }

    fn neighbors(&self, p: &Point) -> Neighbors<'_> {
        Neighbors::new(self, *p)
    }

    fn birth_cell(&mut self, p: &Point) {
        self.board.insert(*p);
    }

    fn kill_cell(&mut self, p: &Point) {
        self.board.remove(p);
    }

    fn iter(&self) -> impl Iterator<Item = &Point> + '_ {
        self.board.iter()
    }

    fn window(
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

struct BoardWidget<'b> {
    board: &'b Board,
    origin: Point,
    // TODO: zoom
}

impl<'b> BoardWidget<'b> {
    fn new(board: &'b Board) -> Self {
        BoardWidget {
            board,
            origin: Default::default(),
        }
    }

    fn pan(&mut self, delta: Point) {
        self.origin += delta;
    }
}

impl<'b> Widget for BoardWidget<'b> {
    fn render(self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        for (_point, dx, dy) in self.board.window(
            self.origin - Point::x(area.width as i64 / 2) - Point::y(area.height as i64 / 2),
            area.width,
            area.height,
        ) {
            buf.get_mut(dx, dy).set_symbol(tui::symbols::bar::FULL);
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

// Contains board and any game parameters
// Game of Life Rules:
// 1. Any live cell with fewer than two live neighbours dies (referred to as underpopulation)
// 2. Any live cell with more than three live neighbours dies (referred to as overpopulation)
// 3. Any live cell with two or three live neighbours lives, unchanged, to the next generation
// 4. Any dead cell with exactly three live neighbours comes to life
#[derive(Debug, Default)]
struct GameOfLife {
    board: Board,
    killed_cells: HashSet<Point>,
    birthed_cells: HashSet<Point>,
    generation: u32,
}

impl<const N: usize> From<[Point; N]> for GameOfLife {
    fn from(value: [Point; N]) -> Self {
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

// Contains game, user config, UI state, handles events
struct App {
    game: GameOfLife,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, DisableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut game = ".xxx
xxx."
        .parse::<GameOfLife>()?;
    println!("{:?}", game.board);

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let board = BoardWidget::new(&game.board);
            f.render_widget(board, size);
        })?;

        if poll(Duration::from_millis(750))? {
            if let crossterm::event::Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                ..
            }) = read()?
            {
                break;
            }
        }

        game.step();
        // println!("{:?}", game.board);
    }

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
