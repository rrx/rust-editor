#![feature(partition_point)]
#![feature(assoc_char_funcs)]
#![feature(atomic_fetch_update)]

//pub mod frontend;
pub mod text;
//pub mod frontend_crossterm;
//pub mod frontend_debug;
pub mod bindings;
//pub mod ism;
pub mod cli;

//pub fn gui(buf: &mut text::TextBuffer) {
    //let mut fe = frontend_crossterm::FrontendCrossterm::new();
    //fe.read_loop(buf);
//}

//pub fn debug(buf: &mut text::TextBuffer) {
    //let mut fe = frontend_debug::FrontendDebug::new();
    //buf.set_size(20, 10);
    //ism::process(&mut fe, buf);
//}

