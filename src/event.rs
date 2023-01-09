use std::{
    sync::{
        atomic::AtomicBool,
        mpsc::{channel, Iter, Receiver, Sender},
        Arc, RwLock,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crossterm::event::{
    poll, read,
    Event::{Key, Mouse},
    KeyEvent, MouseEvent,
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
    pub thread: JoinHandle<()>,
}

impl IoProducer {
    pub fn spawn(sender: EventSender) -> Self {
        let thread = thread::spawn(move || loop {
            match read() {
                Ok(Key(e)) => sender.send(e.into()).unwrap(),
                Ok(Mouse(e)) => sender.send(e.into()).unwrap(),
                _ => (),
            };
        });
        Self { thread }
    }
}

pub struct Timer {
    pub thread: JoinHandle<()>,
    lock: Arc<RwLock<Duration>>,
}

impl Timer {
    pub fn spawn(sender: EventSender, period: Duration, event: Event) -> Self {
        let lock = Arc::new(RwLock::new(period));
        let thread = {
            let lock = lock.clone();
            thread::spawn(move || {
                let mut tick_rate = period;
                loop {
                    sender.send(event).ok();
                    if let Ok(new_tick_rate) = lock.try_read() {
                        tick_rate = *new_tick_rate;
                    }
                    thread::sleep(tick_rate);
                }
            })
        };

        Self { thread, lock }
    }

    pub fn set_period(&self, period: Duration) {
        *self.lock.write().unwrap() = period;
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Render,
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
