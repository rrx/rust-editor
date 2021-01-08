use log::*;
use std::time::Duration;
use crossterm::{
    event::{
        poll, read, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseEvent, MouseEventKind},
};

pub trait FrontendTrait {
    fn reset(&mut self);
    fn render(&mut self, commands: Vec<DrawCommand>, fsm: &InputStateMachine);
}

#[derive(Debug)]
pub enum DrawCommand {
    Clear(u16),
    Line(u16, usize, String),
    Status(u16, String),
    Cursor(u16, u16)
}

#[derive(PartialEq, Debug)]
pub enum ReadEvent {
    Stop,
    Mouse(u16, u16),
    Scroll(i16),
    ScrollPage(f32),
    Line(i64),
    LineNav(i32),
    Resize(u16,u16),
    MoveCursorY(i32),
    MoveCursorX(i32)
}

#[derive(PartialEq, Debug)]
pub enum InputState {
    Start
}

#[derive(PartialEq, Debug)]
pub enum InputMode {
    Insert,
    Normal,
    Command
}

#[derive(PartialEq, Debug)]
pub struct InputStateMachine {
    state: InputState,
    mode: InputMode,
    number: String,
    events: Vec<ReadEvent>
}

impl InputStateMachine {
    pub fn new() -> Self {
        Self {
            state: InputState::Start,
            mode: InputMode::Normal,
            number: String::new(),
            events: Vec::new()
        }
    }

    pub fn add(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match self.mode {
            InputMode::Normal => {
                self.add_normal(code, modifiers);
            }
            _ => ()
        }
    }

    pub fn queue(&mut self, event: ReadEvent) {
        self.events.push(event);
    }

    pub fn read(&mut self) -> Vec<ReadEvent> {
        self.events.drain(0..).collect()
    }

    pub fn add_normal(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        if modifiers == KeyModifiers::CONTROL {
            match code {
                KeyCode::Char('a') => self.queue(ReadEvent::LineNav(0)),
                KeyCode::Char('e') => self.queue(ReadEvent::LineNav(-1)),
                KeyCode::Char('u') => self.queue(ReadEvent::ScrollPage(-0.5)),
                KeyCode::Char('d') => self.queue(ReadEvent::ScrollPage(0.5)),
                KeyCode::Char('f') => self.queue(ReadEvent::ScrollPage(1.)),
                KeyCode::Char('b') => self.queue(ReadEvent::ScrollPage(-1.)),
                _ => {}
            }
        } else {
            match code {
                KeyCode::Char('q') => self.queue(ReadEvent::Stop),
                KeyCode::Char('j') => self.queue(ReadEvent::MoveCursorY(1)),
                KeyCode::Char('k') => self.queue(ReadEvent::MoveCursorY(-1)),
                KeyCode::Char('h') => self.queue(ReadEvent::MoveCursorX(-1)),
                KeyCode::Char('l') => self.queue(ReadEvent::MoveCursorX(1)),
                KeyCode::Char('n') => self.queue(ReadEvent::Scroll(1)),
                KeyCode::Char('p') => self.queue(ReadEvent::Scroll(-1)),
                KeyCode::Char('g') => self.queue(ReadEvent::Line(0)),
                KeyCode::Char('G') => self.queue(ReadEvent::Line(-1)),
                KeyCode::Char(number) if number >= '0' && number <= '9' => {
                    self.number.push(number);
                }

                _ => {}
            }
        }
    }

    pub fn term_event_process(&mut self, evt: Event) -> Vec<ReadEvent> {
        let mut out = Vec::new();
        //info!("{:?}", evt);
        match evt {
            Event::Resize(width, height) => out.push(ReadEvent::Resize(width, height)),
            Event::Key(KeyEvent { code, modifiers }) => {
                self.add(code, modifiers);
                out.append(&mut self.read());
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
        //info!("{:?}", out);
        out
    }
}

pub fn read_loop(fe: &mut dyn FrontendTrait, buf: &mut crate::text::TextBuffer) {
    fe.reset();
    let mut fsm = InputStateMachine::new();
    fe.render(buf.render_view(), &fsm);
    loop {
        if poll(Duration::from_millis(1_000)).unwrap() {
            let evt = read().unwrap();
            for read_event in fsm.term_event_process(evt) {
                if read_event == ReadEvent::Stop {
                    info!("Stop");
                    return;
                }
                buf.command(read_event)
            }
            fe.render(buf.render_view(), &fsm);
        }
    }
}

