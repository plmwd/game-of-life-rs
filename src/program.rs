use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
    io,
    time::Duration,
};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::CrosstermBackend, layout::Rect};

use crate::event::{IoProducer, Listener};
use crate::{
    command::Command::{self, Chain, Exit, RegisterHitbox},
    model::Model,
    terminal::{within, Terminal},
};

// TODO: Timer commands
// Timer (one-shot and periodic) commands which take a Duration and Fn |&mut Model|. They don't
// generate any events (?) since they're essentially an async Mode::update. All timer state and
// logic is maintained by Program.
pub struct Program<Id> {
    hitboxes: BTreeMap<Id, Rect>,
    tick_rate: Duration,
}

#[derive(Debug)]
pub enum ProgramError {
    Io(io::Error),
    EventRecv,
}

impl Display for ProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: better impl
        Debug::fmt(&self, f)
    }
}

impl std::error::Error for ProgramError {}

pub struct Context<'a, Id> {
    prog: &'a Program<Id>,
}

impl<'a, Id: Copy + Ord> Context<'a, Id> {
    fn new(prog: &'a Program<Id>) -> Self {
        Context { prog }
    }

    pub fn find_hitbox(&self, x: u16, y: u16) -> Option<(Id, Rect)> {
        self.prog
            .hitboxes
            .iter()
            .find(|(_id, rect)| within(rect, x, y))
            .map(|(id, rect)| (*id, *rect))
    }
}

impl From<io::Error> for ProgramError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<std::sync::mpsc::RecvError> for ProgramError {
    fn from(_value: std::sync::mpsc::RecvError) -> Self {
        ProgramError::EventRecv
    }
}

pub type ProgramResult = Result<(), ProgramError>;

impl<Id: Ord + Copy> Program<Id> {
    pub fn new() -> Self {
        Self {
            hitboxes: Default::default(),
            tick_rate: Duration::from_millis(15),
        }
    }

    pub fn tick(mut self, tick_rate: Duration) -> Self {
        self.tick_rate = tick_rate;
        self
    }

    pub fn run<M: Model<Id = Id>>(mut self, mut model: M) -> ProgramResult {
        let mut stdout = io::stdout();
        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let listener: Listener = Listener::default();
        let io_producer = IoProducer::spawn(listener.subscribe(), self.tick_rate);

        let ret = self.run_event_loop(&mut terminal, listener, &mut model);

        io_producer.kill();
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        ret
    }

    fn run_event_loop<M: Model<Id = Id>>(
        &mut self,
        terminal: &mut Terminal,
        listener: Listener,
        model: &mut M,
    ) -> ProgramResult {
        loop {
            let mut cx = Context::new(self);
            let event = listener.next()?;
            let cmd = model.update(&mut cx, event);
            match cmd {
                Some(Command::Exit) => break,
                Some(cmd) => self.execute_cmd(cmd),
                _ => (),
            };
            let mut cmd = None;
            terminal.draw(|f| cmd = model.view(f))?;
            match cmd {
                Some(Command::Exit) => break,
                Some(cmd) => self.execute_cmd(cmd),
                _ => (),
            };
        }

        Ok(())
    }

    fn execute_cmd(&mut self, cmd: Command<Id>) {
        match cmd {
            Chain(cmds) => {
                for cmd in cmds {
                    self.execute_cmd(cmd);
                }
            }
            RegisterHitbox(id, area) => {
                self.hitboxes.insert(id, area);
            }
            Exit => (),
            _ => (),
        };
    }
}
