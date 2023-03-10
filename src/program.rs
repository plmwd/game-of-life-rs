use std::{
    fmt::{Debug, Display},
    io,
    time::Duration,
};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::backend::CrosstermBackend;

use crate::event::{Event, IoProducer, Listener, Timer};
use crate::{model::Model, terminal::Terminal};

// TODO: Timer commands
// Timer (one-shot and periodic) commands which take a Duration and Fn |&mut Model|. They don't
// generate any events (?) since they're essentially an async Mode::update. All timer state and
// logic is maintained by Program.
pub struct Program {
    tick_rate: Duration,
}

type ComponentId = u64;

#[derive(Debug, Clone)]
pub enum Command {
    SetTickRate(Duration),
    Exit,
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

#[derive(Debug, Default)]
pub struct Context {
    cmds: Vec<Command>,
}

impl Context {
    pub fn run(&mut self, cmd: Command) {
        self.cmds.push(cmd);
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

impl Program {
    pub fn new() -> Self {
        Self {
            tick_rate: Duration::from_millis(15),
        }
    }

    pub fn tick(mut self, tick_rate: Duration) -> Self {
        self.tick_rate = tick_rate;
        self
    }

    pub fn run<M: Model>(mut self, mut model: M) -> ProgramResult {
        let mut stdout = io::stdout();
        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let ret = self.run_event_loop(&mut terminal, &mut model);

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        ret
    }

    fn run_event_loop<M: Model>(
        &mut self,
        terminal: &mut Terminal,
        model: &mut M,
    ) -> ProgramResult {
        let mut cx = Context::default();
        let listener: Listener = Listener::default();
        let _io_producer = IoProducer::spawn(listener.subscribe());
        let tick_producer =
            Timer::spawn(listener.subscribe(), Duration::from_millis(50), Event::Tick);
        let _render_tick_producer = Timer::spawn(
            listener.subscribe(),
            Duration::from_millis(15),
            Event::Render,
        );

        let execute_cmd = |cmd: &Command| {
            if let Command::SetTickRate(dur) = cmd {
                tick_producer.set_period(*dur);
            }
        };

        loop {
            let event = listener.next()?;
            model.update(&mut cx, event);
            for cmd in &cx.cmds {
                match cmd {
                    Command::Exit => return Ok(()),
                    cmd => execute_cmd(cmd),
                }
            }
            terminal.draw(|f| model.view(&mut cx, f))?;
            for cmd in &cx.cmds {
                match cmd {
                    Command::Exit => return Ok(()),
                    cmd => execute_cmd(cmd),
                }
            }
        }
    }
}
