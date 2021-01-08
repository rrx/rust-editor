use log::*;
use std::time::Duration;
use crossterm::{
    event::{
        poll, read, Event, KeyCode, KeyEvent,
        MouseEvent, MouseEventKind},
};

pub trait FrontendTrait {
    fn reset(&mut self);
    fn render(&mut self, commands: Vec<DrawCommand>);
}

#[derive(Debug)]
pub enum DrawCommand {
    Clear(u16),
    Line(u16, usize, String),
    Status(u16, String),
    Cursor(u16, u16)
}

#[derive(PartialEq)]
pub enum ReadEvent {
    Stop,
    Mouse(u16, u16),
    Scroll(i16),
    Line(i64),
    Resize(u16,u16),
    MoveCursorY(i32),
    MoveCursorX(i32)
}

pub fn term_event_process(evt: Event) -> Vec<ReadEvent> {
    let mut out = Vec::new();
    match evt {
        Event::Resize(width, height) => out.push(ReadEvent::Resize(width, height)),
        Event::Key(KeyEvent { code, .. }) => {
            match code {
                KeyCode::Char('q') => out.push(ReadEvent::Stop),
                KeyCode::Char('j') => out.push(ReadEvent::MoveCursorY(1)),
                KeyCode::Char('k') => out.push(ReadEvent::MoveCursorY(-1)),
                KeyCode::Char('h') => out.push(ReadEvent::MoveCursorX(-1)),
                KeyCode::Char('l') => out.push(ReadEvent::MoveCursorX(1)),
                KeyCode::Char('n') => out.push(ReadEvent::Scroll(1)),
                KeyCode::Char('p') => out.push(ReadEvent::Scroll(-1)),
                KeyCode::Char('g') => out.push(ReadEvent::Line(0)),
                KeyCode::Char('G') => out.push(ReadEvent::Line(-1)),
                _ => {}
            }
        },
        Event::Mouse(MouseEvent {kind, column, row, modifiers}) => {
            match kind {
                MouseEventKind::ScrollUp => {
                    out.push(ReadEvent::Scroll(1));
                }
                MouseEventKind::ScrollDown => {
                    out.push(ReadEvent::Scroll(-1));
                }
                MouseEventKind::Moved => {
                    out.push(ReadEvent::Mouse(column, row));
                }
                _ => ()
            }
        }
        _ => ()
    };
    out
}

pub fn read_loop(fe: &mut dyn FrontendTrait, buf: &mut crate::text::TextBuffer) {
    fe.reset();
    fe.render(buf.render_view());
    loop {
        if poll(Duration::from_millis(1_000)).unwrap() {
            let evt = read().unwrap();
            for read_event in term_event_process(evt) {
                if read_event == ReadEvent::Stop {
                    info!("Stop");
                    return;
                }
                buf.command(read_event)
            }
            fe.render(buf.render_view());
        }
    }
}

