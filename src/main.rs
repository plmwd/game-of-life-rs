#![allow(dead_code)]

mod adapter;
mod command;
mod event;
mod model;
mod program;

use crossterm::{
    event::{
        poll, read, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers, MouseButton,
        MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    collections::HashSet,
    fmt::Display,
    fs::{self, File},
    io::{self, Write},
    str::FromStr,
    time::{Duration, Instant},
};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    symbols::DOT,
    text::{Spans, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Tabs, Widget},
    Terminal,
};

const LOG_FILE_NAME: &str = "gameoflife.log";

// TODO: reorganize project

fn log(s: &str) {
    let mut temp_file = std::env::temp_dir();
    temp_file.push(LOG_FILE_NAME);

    if let Ok(mut f) = File::options()
        .append(true)
        .create(true)
        .write(true)
        .open(temp_file)
    {
        f.write(format!("[{:?}]: {}\n", Instant::now(), s).as_ref())
            .ok();
    };
}

fn get_logs() -> Vec<String> {
    let mut temp_file = std::env::temp_dir();
    temp_file.push(LOG_FILE_NAME);
    let mut logs = Vec::default();

    if let Ok(s) = fs::read_to_string(temp_file) {
        for line in s.lines().rev() {
            logs.push(line.to_owned());
        }
    }

    logs
}

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

    fn toggle_cell(&mut self, p: &Point) {
        if self.board.contains(p) {
            self.kill_cell(p);
        } else {
            self.birth_cell(p);
        }
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
    block: Option<Block<'b>>,
    // TODO: zoom
}

impl<'b> BoardWidget<'b> {
    fn new(board: &'b Board) -> Self {
        BoardWidget {
            board,
            origin: Default::default(),
            block: None,
        }
    }

    fn pan_to(mut self, origin: Point) -> Self {
        self.origin = origin;
        self
    }

    fn block(mut self, block: Block<'b>) -> Self {
        self.block = Some(block);
        self
    }
}

impl<'b> Widget for BoardWidget<'b> {
    fn render(mut self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        let board_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        buf.set_style(board_area, Style::default().bg(Color::LightBlue));

        for (_point, dx, dy) in self.board.window(
            self.origin - Point::new(board_area.width as i64 / 2, board_area.height as i64 / 2),
            board_area.width,
            board_area.height,
        ) {
            buf.get_mut(board_area.x + dx, board_area.y + dy)
                .set_symbol(tui::symbols::bar::FULL);
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

// TODO: use type-state to track game state?
impl GameOfLife {
    fn board_from_str(&mut self, s: &str) -> Result<(), GameError> {
        self.board = s.parse()?;
        Ok(())
    }

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

#[derive(Debug, Default)]
enum AppState {
    #[default]
    Stopped,
    Paused,
    Running,
}

impl AppState {
    fn toggle(&mut self) {
        match self {
            AppState::Stopped => *self = AppState::Running,
            AppState::Paused => *self = AppState::Running,
            AppState::Running => *self = AppState::Paused,
        }
    }
}

#[derive(Debug, Default)]
enum AppView {
    #[default]
    Game,
    Logs,
}

// Contains game, user config, UI state, handles events
#[derive(Debug)]
struct App {
    game: GameOfLife,
    tick_rate: Duration,
    origin: Point,
    state: AppState,
    view: AppView,
}

impl App {
    fn new(tick_rate: Duration) -> Self {
        App {
            tick_rate,
            game: Default::default(),
            origin: Default::default(),
            state: Default::default(),
            view: Default::default(),
        }
    }
}

// TODO: this is all getting a little ugly
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(Duration::from_millis(750));
    let init_board = "x..x.\n....x\nx...x\n.xxxx";
    app.game.board_from_str(init_board)?;

    let mut step_time = Duration::default();
    let mut frame_time = Duration::default();
    let mut print_info = true;

    loop {
        let frame_start = Instant::now();

        terminal.draw(|f| {
            if print_info {
                print_info = false;
            }
            let board = BoardWidget::new(&app.game.board)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::White)),
                )
                .pan_to(app.origin);
            let generation =
                Paragraph::new(Text::from(format!("generation = {}", app.game.generation)));
            let tick_rate = Paragraph::new(Text::from(format!("tick rate = {:?}", app.tick_rate)));
            let state = Paragraph::new(Text::from(format!("state = {:?}", app.state)));

            let titles = ["Game", "Logs"].iter().cloned().map(Spans::from).collect();
            let tabs = Tabs::new(titles)
                .block(Block::default().title("Tabs").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Yellow))
                .divider(DOT);

            let tabs = match app.view {
                AppView::Game => tabs.select(0),
                AppView::Logs => tabs.select(1),
            };

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Percentage(100)])
                .split(f.size());

            let tabs_area = chunks[0];
            let main_area = chunks[1];

            f.render_widget(tabs, tabs_area);
            match app.view {
                AppView::Game => {
                    let chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Length(25), Constraint::Min(0)])
                        .split(main_area);
                    let info_panel_area = chunks[0];
                    let board_area = chunks[1];

                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(4); 6])
                        .split(info_panel_area);

                    let generation_area = chunks[0];
                    let tick_rate_area = chunks[1];
                    let state_area = chunks[2];
                    let frame_time_area = chunks[3];
                    let step_time_area = chunks[4];
                    let origin_area = chunks[5];

                    f.render_widget(generation, generation_area);
                    f.render_widget(tick_rate, tick_rate_area);
                    f.render_widget(state, state_area);
                    f.render_widget(board, board_area);
                    f.render_widget(
                        Paragraph::new(Text::from(format!("origin = \n{:?}", app.origin))),
                        origin_area,
                    );
                    f.render_widget(
                        Paragraph::new(Text::from(format!("frame time = {:?}", frame_time))),
                        frame_time_area,
                    );
                    f.render_widget(
                        Paragraph::new(Text::from(format!("step time = {:?}", step_time))),
                        step_time_area,
                    );
                }
                AppView::Logs => {
                    f.render_widget(Paragraph::new(Text::from(get_logs().join("\n"))), main_area);
                }
            };
        })?;

        if poll(app.tick_rate)? {
            match read()? {
                crossterm::event::Event::Key(key) => {
                    match key.code {
                        KeyCode::Char('1') => app.view = AppView::Game,
                        KeyCode::Char('2') => app.view = AppView::Logs,
                        KeyCode::Char('q') => break,
                        KeyCode::Char('d') => {
                            app.tick_rate = app.tick_rate.saturating_sub(Duration::from_millis(50))
                        }
                        KeyCode::Char('u') => {
                            app.tick_rate = app.tick_rate.saturating_add(Duration::from_millis(50))
                        }
                        KeyCode::Char('h') => app.origin.dx(-1),
                        KeyCode::Char('j') => app.origin.dy(-1),
                        KeyCode::Char('k') => app.origin.dy(1),
                        KeyCode::Char('l') => app.origin.dx(1),
                        KeyCode::Char(' ') => app.state.toggle(),
                        KeyCode::Char('r') => {
                            app.state = AppState::Stopped;
                            app.game.board_from_str(init_board)?;
                            app.game.generation = 0;
                            app.origin = Default::default();
                        }
                        _ => {}
                    };
                }
                crossterm::event::Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column,
                    row,
                    modifiers: KeyModifiers::NONE,
                }) => {}
                _ => (),
            }
        } else {
            let step_start = Instant::now();
            if let AppState::Running = app.state {
                app.game.step();
            }
            step_time = Instant::now() - step_start;
        }

        frame_time = Instant::now() - frame_start;
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
