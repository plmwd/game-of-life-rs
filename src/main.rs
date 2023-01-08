#![allow(dead_code)]

mod board;
mod event;
mod game;
mod model;
mod point;
mod program;
mod terminal;
mod widgets;

use board::Board;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use event::Event;
use game::{GameError, GameOfLife};
use model::Model;
use point::Point;
use program::{Command, Context, Program};
use std::time::Duration;
use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Text,
    widgets::Paragraph,
};
use widgets::BoardWidget;

// TODO:
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

const QUEEN_BEE_BOARD: &str = "xx..\nx.x.\n...x\nx..x\n...x\nx.x.\nxx..";

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
}

// Contains game, user config, UI state, handles events
#[derive(Debug)]
struct App {
    game: GameOfLife,
    game_tick: Duration,
    origin: Point,
    state: AppState,
    view: AppView,
    mouse: (u16, u16),
    board_area: Rect,
    initial_board: Board,
}

impl App {
    fn new(game_tick: Duration) -> Self {
        App {
            game_tick,
            game: Default::default(),
            origin: Default::default(),
            state: Default::default(),
            view: Default::default(),
            board_area: Default::default(),
            mouse: Default::default(),
            initial_board: Default::default(),
        }
    }

    fn board(mut self, s: &str) -> Result<Self, GameError> {
        self.game.board_from_str(s)?;
        Ok(self)
    }
}

#[derive(Debug, Copy, Clone, Ord, Eq, PartialEq, PartialOrd)]
enum ComponentId {
    Board,
}

fn contains(rect: Rect, x: u16, y: u16) -> Option<(u16, u16)> {
    if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
        return Some((x - rect.x, y - rect.y));
    }
    None
}

impl Model for App {
    fn update(&mut self, cx: &mut Context, event: Event) {
        // TODO: this is unreadable
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char(' '),
                ..
            }) => {
                if matches!(self.state, AppState::Stopped) {
                    self.initial_board = self.game.board.clone();
                }
                self.state.toggle();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                self.state = AppState::Stopped;
                self.game.generation = 0;
                self.game.board = self.initial_board.clone();
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => cx.run(Command::Exit),
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                if matches!(self.state, AppState::Stopped) {
                    self.game.board.clear();
                }
            }
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column,
                row,
                modifiers: KeyModifiers::NONE,
            }) => {
                if matches!(self.state, AppState::Stopped) {
                    if let Some((rel_x, rel_y)) = contains(self.board_area, column, row) {
                        let board_x =
                            rel_x as i64 - self.board_area.width as i64 / 2 + self.origin.x;
                        let board_y =
                            rel_y as i64 - self.board_area.height as i64 / 2 + self.origin.y;
                        self.game.board.toggle_cell(&Point::new(board_x, board_y));
                    }
                }
            }
            Event::Tick => {
                if matches!(self.state, AppState::Running) {
                    self.game.step();
                }
            }
            _ => (),
        };
    }

    fn view(&mut self, _cx: &mut Context, f: &mut terminal::Frame) {
        let board = BoardWidget::new(&self.game.board).pan_to(self.origin);
        let generation =
            Paragraph::new(Text::from(format!("generation = {}", self.game.generation)));
        let tick_rate = Paragraph::new(Text::from(format!("tick rate = {:?}", self.game_tick)));
        let state = Paragraph::new(Text::from(format!("state = {:?}", self.state)));

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(25), Constraint::Min(0)])
            .split(f.size());
        let info_panel_area = chunks[0];
        let board_area = chunks[1];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4); 7])
            .split(info_panel_area);

        let generation_area = chunks[0];
        let tick_rate_area = chunks[1];
        let state_area = chunks[2];
        let origin_area = chunks[4];
        // let click_area = chunks[5];
        let mouse_area = chunks[6];
        self.board_area = board_area;

        f.render_widget(generation, generation_area);
        f.render_widget(tick_rate, tick_rate_area);
        f.render_widget(state, state_area);
        f.render_widget(board, board_area);
        f.render_widget(
            Paragraph::new(Text::from(format!("origin = \n{:?}", self.origin))),
            origin_area,
        );
        f.render_widget(
            Paragraph::new(Text::from(format!("mouse = {:?}", self.mouse))),
            mouse_area,
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = App::new(Duration::from_millis(75)).board(QUEEN_BEE_BOARD)?;
    Program::new().run(app)?;
    Ok(())
}
