pub mod frontend;
pub mod text;
pub mod frontend_termion;
pub mod frontend_crossterm;

pub fn gui(buf: &mut text::TextBuffer) {
    //let mut fe = frontend_termion::FrontendTermion::new();
    let mut fe = frontend_crossterm::FrontendCrossterm::new();
    fe.read_loop(buf);
}
