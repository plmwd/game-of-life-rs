use crate::{command::Command, event::Event, program::Context, terminal::Frame};

pub trait Model {
    type Id;

    fn update(&mut self, cx: &mut Context<Self::Id>, event: Event) -> Option<Command<Self::Id>>;
    fn view(&self, f: &mut Frame) -> Option<Command<Self::Id>>;
}
