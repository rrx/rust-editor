pub mod frontend;
pub mod text;
pub mod frontend_crossterm;
pub mod frontend_debug;
pub mod scroll;

pub fn gui(buf: &mut text::TextBuffer) {
    let mut fe = frontend_crossterm::FrontendCrossterm::new();
    fe.read_loop(buf);
}

pub fn debug(buf: &mut text::TextBuffer) {
    let mut fe = frontend_debug::FrontendDebug::new();
    buf.set_size(20, 10);
    frontend::read_loop(&mut fe, buf);
}

