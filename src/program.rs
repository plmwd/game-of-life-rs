use std::{
    collections::HashMap,
    io::{self, Stdout},
};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::CrosstermBackend, layout::Rect};

use crate::{adapter::Terminal, model::Model};

struct Program<M: Model<I, Msg = Msg>, I, Msg> {
    model: M,
    areas: HashMap<I, Vec<Rect>>,
}

enum ProgramError {
    Io(io::Error),
}

impl From<io::Error> for ProgramError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

type ProgramResult = Result<(), ProgramError>;

impl<M: Model<I, Msg = Msg>, I, Msg> Program<M, I, Msg> {
    pub fn new(model: M) -> Self {
        Self {
            model,
            areas: Default::default(),
        }
    }

    pub fn run(mut self) -> ProgramResult {
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        Ok(())
    }
}
