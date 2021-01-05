
use termion::event::{Key, Event, MouseEvent, MouseButton};
use termion::input::{TermRead, MouseTerminal};
use termion::raw::IntoRawMode;
use termion::cursor::{self, DetectCursorPos};
use termion::terminal_size;
use std::fs::File;
use std::io::{Write, stdout, stdin};
use crate::frontend::{DrawCommand, FrontendTrait, ReadEvent};

pub struct FrontendTermion {
    stdout: MouseTerminal<termion::raw::RawTerminal<std::io::Stdout>>,
}

impl FrontendTermion {
    pub fn new() -> Self {
        if !termion::is_tty(&File::create("/dev/stdout").unwrap()) {
            panic!("Not a tty");
        }

        Self {
            stdout: MouseTerminal::from(stdout().into_raw_mode().unwrap())
        }
    }

    pub fn read_loop(&mut self, buf: &mut crate::text::TextBuffer) {
        self.reset();
        let (sx, sy) = terminal_size().unwrap();
        self.render(buf.generate_commands(sx, sy));
        let stdin = stdin();
        let mut stdin = stdin.lock();
        for c in stdin.events() {
            let evt = c.unwrap();
            //eprintln!("ev: {:?}", evt);
            for read_event in self.handle_event(evt) {
                if read_event == ReadEvent::Stop {
                    eprintln!("Stop");
                    return;
                }
                buf.handle_event(read_event)
            }
            let (sx, sy) = terminal_size().unwrap();
            self.render(buf.generate_commands(sx, sy));
        }
    }

    fn handle_event(&self, evt: Event) -> Vec<ReadEvent> {
        let mut out = Vec::new();
        match evt {
            Event::Key(Key::Char('q')) => out.push(ReadEvent::Stop),
            Event::Key(Key::Char('j')) => out.push(ReadEvent::Scroll(1)),
            Event::Key(Key::Char('k')) => out.push(ReadEvent::Scroll(-1)),
            Event::Key(Key::Char('g')) => out.push(ReadEvent::Line(1)),
            Event::Key(Key::Char('G')) => out.push(ReadEvent::Line(-1)),

            Event::Key(Key::Char(c)) => {},
            //Key::Alt(c) => println!("^{}", c),
            //Key::Ctrl(c) => println!("*{}", c),
            //Event::Key(Key::Esc) => out.push(ReadEvent::Stop),
            //Key::Left => println!("←"),
            //Key::Right => println!("→"),
            //Key::Up => println!("↑"),
            //Key::Down => println!("↓"),
            //Key::Backspace => println!("×"),
            Event::Mouse(me) => {
                match me {
                    MouseEvent::Press(btn, a, b) if btn == MouseButton::WheelUp => {
                        out.push(ReadEvent::Scroll(1));
                    }
                    MouseEvent::Press(btn, a, b) if btn == MouseButton::WheelDown => {
                        out.push(ReadEvent::Scroll(-1));
                    }
                    MouseEvent::Press(_, a, b) |
                    MouseEvent::Release(a, b) |
                    MouseEvent::Hold(a, b) => {
                        out.push(ReadEvent::Mouse(a, b));
                    }
                    _ => ()
                }
            }
            _ => {}
        };
        out
    }
}

impl FrontendTrait for FrontendTermion {
    fn reset(&mut self) {
        write!(self.stdout, "{}", termion::clear::All).unwrap();
        write!(self.stdout, "{}", termion::cursor::Goto(1,1)).unwrap();
        self.stdout.flush().unwrap();
    }

    fn render(&mut self, commands: Vec<DrawCommand>) {
        write!(self.stdout, "{}", termion::cursor::Hide).unwrap();
        for command in commands {
            match command {
                DrawCommand::Line(row, s) => {
                    write!(self.stdout,
                        "{}{}{}",
                        termion::cursor::Goto(1, row),
                        termion::clear::CurrentLine,
                        s
                    ).unwrap();


                },
                DrawCommand::Clear(row) => {
                    write!(self.stdout,
                        "{}{}",
                        termion::cursor::Goto(1, row),
                        termion::clear::CurrentLine,
                    ).unwrap();
                }
                DrawCommand::Cursor(a, b) => {
                    write!(self.stdout, "{}", termion::cursor::Goto(a, b));
                }
            }
        }

        //eprintln!("G: {}:{}", p.0, p.1);
        write!(self.stdout, "{}", termion::cursor::Show).unwrap();
        self.stdout.flush().unwrap();
    }
}


