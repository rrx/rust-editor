use log::*;
use crossterm::{
    event::{
        Event, KeyCode, KeyEvent, KeyModifiers,
        MouseEvent, MouseEventKind},
};
use std::convert::TryInto;
use crate::text::display::DrawCommand;
use crate::text::*;


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
    events: Vec<Command>
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

    pub fn queue(&mut self, event: Command) {
        self.events.push(event);
    }

    pub fn read(&mut self) -> Vec<Command> {
        self.events.drain(0..).collect()
    }

    pub fn add_normal(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        if modifiers == KeyModifiers::CONTROL {
            match code {
                KeyCode::Char('a') => self.queue(Command::LineNav(0)),
                KeyCode::Char('e') => self.queue(Command::LineNav(-1)),
                KeyCode::Char('u') => self.queue(Command::ScrollPage(-2)),
                KeyCode::Char('d') => self.queue(Command::ScrollPage(2)),
                KeyCode::Char('f') => self.queue(Command::ScrollPage(1)),
                KeyCode::Char('b') => self.queue(Command::ScrollPage(-1)),
                _ => {}
            }
        } else {
            match code {
                KeyCode::Char('q') => self.queue(Command::Quit),
                KeyCode::Char('j') => self.queue(Command::MoveCursorY(1)),
                KeyCode::Char('k') => self.queue(Command::MoveCursorY(-1)),
                KeyCode::Char('h') => self.queue(Command::MoveCursorX(-1)),
                KeyCode::Char('l') => self.queue(Command::MoveCursorX(1)),
                KeyCode::Char('n') => self.queue(Command::Scroll(1)),
                KeyCode::Char('p') => self.queue(Command::Scroll(-1)),
                KeyCode::Char('g') => self.queue(Command::Line(0)),
                KeyCode::Char('G') => self.queue(Command::Line(-1)),
                KeyCode::Char(number) if number >= '0' && number <= '9' => {
                    self.number.push(number);
                }

                _ => {}
            }
        }
    }

    pub fn term_event_process(&mut self, evt: Event) -> Vec<Command> {
        let mut out = Vec::new();
        //info!("{:?}", evt);
        match evt {
            Event::Resize(width, height) => out.push(Command::Resize(width, height)),
            Event::Key(KeyEvent { code, modifiers }) => {
                self.add(code, modifiers);
                out.append(&mut self.read());
            },
            Event::Mouse(MouseEvent {kind, column, row, modifiers}) => {
                match kind {
                    MouseEventKind::ScrollUp => {
                        out.push(Command::Scroll(1));
                    }
                    MouseEventKind::ScrollDown => {
                        out.push(Command::Scroll(-1));
                    }
                    MouseEventKind::Moved => {
                        out.push(Command::Mouse(column, row));
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

pub trait FrontendTrait {
    fn reset(&mut self);
    fn render(&mut self, commands: Vec<DrawCommand>);
}

pub fn process(fe: &mut dyn FrontendTrait, buf: &mut crate::text::TextBuffer) {
    //let mut q = Vec::new();
    //fe.reset();
    ////let fsm = InputStateMachine::new();
    //fe.render(buf.render_view());
    //loop {
        //let event = crossterm::event::read().unwrap();
        //match event.try_into() {
            //Ok(e) => {
                //q.push(e);
                ////let result = buf.mode.command()(q.as_slice());
                ////match result {
                    //////Ok((_, Command::Quit)) => {
                        //////info!("Quit");
                        //////return;
                    //////}
                    ////Ok((_, x)) => {
                        ////info!("[{:?}] Ok: {:?}\r", &buf.mode, (&q, &x));
                        ////q.clear();
                        //////buf.command(x);
                        ////fe.render(buf.render_view());
                    ////}
                    ////Err(nom::Err::Incomplete(_)) => {
                        ////info!("Incomplete: {:?}\r", (q));
                    ////}
                    ////Err(e) => {
                        ////info!("Error: {:?}\r", (e, &q));
                        ////q.clear();
                    ////}
                ////}
            //}
            //Err(err) => {
                //info!("ERR: {:?}\r", (err));
            //}
        //}
    //}
}

pub fn read_loop(fe: &mut dyn FrontendTrait, buf: &mut crate::text::TextBuffer) {
    fe.reset();
    let mut fsm = InputStateMachine::new();
    fe.render(buf.render_view());
    loop {
        if crossterm::event::poll(std::time::Duration::from_millis(1_000)).unwrap() {
            let evt = crossterm::event::read().unwrap();
            for read_event in fsm.term_event_process(evt) {
                if read_event == Command::Quit {
                    info!("Quit");
                    return;
                }
                buf.command(read_event)
            }
            fe.render(buf.render_view());
        }
    }
}

