pub mod frontend;
pub mod text;
pub mod frontend_termion;

pub fn gui(buf: &mut text::TextBuffer) {
    let mut fe = frontend_termion::FrontendTermion::new();
    fe.read_loop(buf);
}
