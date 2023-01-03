use crossterm::event::{KeyEvent, MouseEvent};

use crate::{adapter::Frame, command::Command, event::Event};

pub trait Model<Id>: Default {
    type Msg;

    fn on(event: Event) -> Self::Msg;
    fn update(msg: Self::Msg) -> Option<Command<Id>>;
    fn view(f: Frame);
}
