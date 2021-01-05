pub trait FrontendTrait {
    fn reset(&mut self);
    fn render(&mut self, commands: Vec<DrawCommand>);
}

#[derive(Debug)]
pub enum DrawCommand {
    Clear(u16),
    Line(u16, String),
    Cursor(u16, u16)
}

#[derive(PartialEq)]
pub enum ReadEvent {
    Stop,
    Mouse(u16, u16),
    Scroll(i16),
    Line(i64),
    Resize(u16,u16)
}
