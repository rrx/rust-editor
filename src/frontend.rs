#[derive(Debug)]
pub enum DrawCommand {
    Clear(u16),
    Line(u16, usize, String),
    Row(u16, u16, String),
    Status(u16, String),
    Cursor(u16, u16)
}


