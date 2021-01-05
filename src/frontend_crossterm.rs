use crossterm::{
    tty::IsTty,
    cursor::{self, position},
    event::{
        poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent,
        KeyModifiers, MouseEvent, MouseEventKind},
    execute, queue, style,
    terminal::{self, ClearType, disable_raw_mode, enable_raw_mode},
    Result,
};
use std::{io::{self, Write, stdout, stdin}, time::Duration};

use crate::frontend::{DrawCommand, FrontendTrait, ReadEvent};

pub struct FrontendCrossterm {
    out: std::io::Stdout
}

impl FrontendCrossterm {
    pub fn new() -> Self {
        if !stdin().is_tty() {
            panic!("Not a tty");
        }

        Self {
            out: stdout()
        }
    }

    pub fn read_loop(&mut self, buf: &mut crate::text::TextBuffer) {
        enable_raw_mode().unwrap();
        execute!(self.out, EnableMouseCapture);
        self.read_loop_inner(buf);
        execute!(self.out, DisableMouseCapture);
        disable_raw_mode().unwrap();
    }

    pub fn read_loop_inner(&mut self, buf: &mut crate::text::TextBuffer) {
        self.reset();
        let (sx, sy) = terminal::size().unwrap();
        eprintln!("s: {}:{}", sx, sy);
        self.render(buf.generate_commands(sx, sy));
        loop {
            if poll(Duration::from_millis(1_000)).unwrap() {
                let evt = read().unwrap();
                for read_event in self.handle_event(evt) {
                    if read_event == ReadEvent::Stop {
                        eprintln!("Stop");
                        return;
                    }
                    buf.handle_event(read_event)
                }
                let (sx, sy) = terminal::size().unwrap();
                eprintln!("s: {}:{}", sx, sy);
                self.render(buf.generate_commands(sx, sy));
            }
        }
    }

    fn handle_event(&self, evt: Event) -> Vec<ReadEvent> {
        let mut out = Vec::new();
        match evt {
            Event::Resize(width, height) => out.push(ReadEvent::Resize(width, height)),
            Event::Key(KeyEvent { code, .. }) => {
                match code {
                    KeyCode::Char('q') => out.push(ReadEvent::Stop),
                    KeyCode::Char('j') => out.push(ReadEvent::Scroll(1)),
                    KeyCode::Char('k') => out.push(ReadEvent::Scroll(-1)),
                    KeyCode::Char('g') => out.push(ReadEvent::Line(1)),
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
}

impl FrontendTrait for FrontendCrossterm {
    fn reset(&mut self) {
        queue!(self.out,
            style::ResetColor,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0,0)
        ).unwrap();
        self.out.flush().unwrap();
    }

    fn render(&mut self, commands: Vec<DrawCommand>) {
        queue!(self.out,
            cursor::Hide,
        ).unwrap();
        for command in commands {
            match command {
                DrawCommand::Line(row, s) => {
                    queue!(self.out,
                        cursor::MoveTo(0, row),
                        terminal::Clear(ClearType::CurrentLine),
                        style::Print(s)
                    ).unwrap();
                },
                DrawCommand::Clear(row) => {
                    queue!(self.out,
                        cursor::MoveTo(0, row),
                        terminal::Clear(ClearType::CurrentLine),
                    ).unwrap();
                }
                DrawCommand::Cursor(a, b) => {
                    queue!(self.out,
                        cursor::MoveTo(a, b),
                    ).unwrap();
                }
            }
        }
        queue!(self.out,
            cursor::Show,
        ).unwrap();
        self.out.flush().unwrap();
    }
}



