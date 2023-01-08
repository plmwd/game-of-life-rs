use crate::{event::Event, program::Context, terminal::Frame};

pub trait Model {
    fn update(&mut self, cx: &mut Context, event: Event);
    fn view(&mut self, cx: &mut Context, f: &mut Frame);
}
