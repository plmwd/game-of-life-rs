use std::{
    fs::File,
    io::{Stderr, Write},
    sync::{
        atomic::AtomicBool,
        mpsc::{channel, Iter, Receiver, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crossterm::{
    event::{
        poll, read,
        Event::{Key, Mouse},
        KeyEvent, MouseEvent,
    },
    terminal::is_raw_mode_enabled,
};

pub type EventSender = Sender<Event>;
pub type EventReceiver = Receiver<Event>;

pub struct Listener {
    sender: EventSender,
    receiver: EventReceiver,
}

impl Default for Listener {
    fn default() -> Self {
        let (sender, receiver) = channel();
        Self { sender, receiver }
    }
}

impl Listener {
    pub fn subscribe(&self) -> EventSender {
        self.sender.clone()
    }

    pub fn iter(&self) -> Iter<'_, Event> {
        self.receiver.iter()
    }

    pub fn next(&self) -> Result<Event, std::sync::mpsc::RecvError> {
        self.receiver.recv()
    }
}

pub struct IoProducer {
    thread: JoinHandle<()>,
    running: Arc<AtomicBool>,
}

impl IoProducer {
    pub fn kill(self) {
        self.running
            .store(false, std::sync::atomic::Ordering::Relaxed)
    }
    pub fn spawn(sender: EventSender, tick_rate: Duration) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let is_running = running.clone();
        let thread = thread::spawn(move || {
            let mut poll_time = tick_rate;
            loop {
                let start = Instant::now();

                if !is_running.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                if poll(poll_time).unwrap() {
                    match read() {
                        Ok(Key(e)) => sender.send(e.into()).unwrap(),
                        Ok(Mouse(e)) => sender.send(e.into()).unwrap(),
                        _ => (),
                    };
                } else {
                    sender.send(Event::Tick).unwrap();
                }

                // Try to tick tick_rate duration
                let duration = Instant::now() - start;
                poll_time = if duration > poll_time {
                    tick_rate
                } else {
                    tick_rate - poll_time
                }
            }
        });
        Self { thread, running }
    }
}

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Tick,
}

impl From<KeyEvent> for Event {
    fn from(value: KeyEvent) -> Self {
        Event::Key(value)
    }
}

impl From<MouseEvent> for Event {
    fn from(value: MouseEvent) -> Self {
        Event::Mouse(value)
    }
}
