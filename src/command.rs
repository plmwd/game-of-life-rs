use tui::layout::Rect;

#[derive(Clone)]
pub enum Command<Id> {
    Chain(Vec<Self>),
    RegisterHitbox(Id, Rect),
    RemoveHitbox(Id),
    Exit,
}
