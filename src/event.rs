use std::{
    sync::mpsc::{channel, Iter, Receiver, Sender},
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
    fn subscribe(&self) -> EventSender {
        self.sender.clone()
    }

    pub fn iter(&self) -> Iter<'_, Event> {
        self.receiver.iter()
    }
}

pub struct IoProducer {
    thread: JoinHandle<()>,
}

impl IoProducer {
    fn spawn(sender: EventSender, tick_rate: Duration) -> Self {
        let thread = thread::spawn(move || {
            let mut poll_time = tick_rate;
            loop {
                let start = Instant::now();

                if poll(poll_time).unwrap() {
                    match read().unwrap() {
                        Key(e) => sender.send(e.into()).unwrap(),
                        Mouse(e) => sender.send(e.into()).unwrap(),
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
        Self { thread }
    }
}

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
