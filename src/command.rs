use tui::layout::Rect;

pub enum Command<Id> {
    Chain(Vec<Self>),
    RegisterArea(Id, Rect),
    Exit,
}
