extern crate ropey;
extern crate termion;
extern crate unicode_segmentation;
extern crate clap;

use std::fs::File;
use std::io;
use std::fs;
use std::vec::Vec;
use clap::{Arg, App};

use ropey::iter::{Bytes, Chars, Chunks, Lines};
use ropey::{Rope, RopeSlice};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

use termion::event::{Key, Event, MouseEvent};
use termion::input::{TermRead, MouseTerminal};
use termion::raw::IntoRawMode;
use termion::cursor::{self, DetectCursorPos};
use termion::terminal_size;
use std::io::{Write, stdout, stdin};
use std::fmt::format;


struct TextBuffer {
    text: Rope,
    path: String,
    dirty: bool,
    row: u32,
    col: u32,
    line_offset: usize
}

impl TextBuffer {
    fn from_path(path: &str) -> io::Result<TextBuffer> {
        let text = Rope::from_reader(&mut io::BufReader::new(File::open(&path)?))?;
        Ok(TextBuffer {
            text: text,
            path: path.to_string(),
            dirty: false,
            row: 1,
            col: 1,
            line_offset: 0
        })
    }

    fn update_cursor(&mut self, row: u32, col: u32) {
        self.row = row;
        self.col = col;
    }

    fn pos(&self) -> (u32, u32) {
        return (self.row, self.col);
    }

    fn get_line<'a>(&'a self, idx: usize) -> RopeSlice<'a> {
        self.text.line(idx)
    }

    fn bytes<'a>(&'a self) -> Bytes<'a> {
        self.text.bytes()
    }

    fn chars<'a>(&'a self) -> Chars<'a> {
        self.text.chars()
    }

    fn lines<'a>(&'a self) -> Lines<'a> {
        self.text.lines()
    }

    fn lines_at<'a>(&'a self, line_inx: usize) -> Lines<'a> {
        self.text.lines_at(line_inx)
    }

    fn chunks<'a>(&'a self) -> Chunks<'a> {
        self.text.chunks()
    }

    fn edit(&mut self, start: usize, end: usize, text: &str) {
        if start != end {
            self.text.remove(start..end);
        }
        if !text.is_empty() {
            self.text.insert(start, text);
        }
        self.dirty = true;
    }
}

#[derive(Debug)]
enum DrawCommand {
    Clear,
    Line(String)
}

#[derive(Debug)]
struct DrawLine {
    row: usize,
    command: DrawCommand
}

fn reset() {
    let mut stdout = stdout().into_raw_mode().unwrap();
    write!(stdout, "{}", termion::clear::All).unwrap();
    write!(stdout, "{}", termion::cursor::Goto(1,1)).unwrap();
    write!(stdout, "{}", termion::cursor::Hide).unwrap();
    stdout.flush().unwrap();
}

fn render(buf: &TextBuffer, commands: Vec<DrawLine>) {
    let mut stdout = stdout().into_raw_mode().unwrap();
    for command in commands {
        match command.command {
            DrawCommand::Line(s) => {
                write!(stdout,
                    "{}{}{}",
                    termion::cursor::Goto(1, command.row as u16),
                    termion::clear::CurrentLine,
                    s
                ).unwrap();
            },
            DrawCommand::Clear => {
                write!(stdout,
                    "{}{}",
                    termion::cursor::Goto(1, command.row as u16),
                    termion::clear::CurrentLine,
                ).unwrap();
            }
        }
    }
    stdout.flush().unwrap();
    let p = buf.pos();
    write!(stdout, "{}", termion::cursor::Goto(p.0 as u16, p.1 as u16)).unwrap();
    let (x, y) = stdout.cursor_pos().unwrap();
    write!(stdout,
       "{}{}Cursor is at: ({},{})",
       cursor::Goto(1, 5),
       termion::clear::UntilNewline,
       x,
       y,
       ).unwrap();


    write!(stdout, "{}{}",
        termion::cursor::Goto(p.0 as u16, p.1 as u16),
        termion::cursor::Show).unwrap();
}

fn generate_commands(buf: &TextBuffer) -> Vec<DrawLine> {
    let mut out = Vec::new();
    let (sx, sy) = terminal_size().unwrap();

    let mut row: usize = 1;
    let mut lines = buf.lines_at(buf.line_offset);

    while row <= sy as usize {
        match lines.next() {
            Some(line) => {
                let mut start = 0;
                let len = line.len_chars();
                while start < len {
                    let mut s = String::with_capacity(sx as usize);
                    let end = start + std::cmp::min(len-start, sx as usize);
                    //println!("start: {}, end: {}, sx: {}, row: {}, len: {}", start, end, sx, row, len);

                    if (end > start) {
                        let s0 = line.slice(start..end);
                        s.insert_str(0, &format!("{}", s0).to_owned());
                        out.push(DrawLine { row: row, command: DrawCommand::Line(s.replace("\n", ".")) });
                        start = end;
                        row += 1;
                    }
                }
            },
            None => {
                out.push(DrawLine { row: row, command: DrawCommand::Line(";".to_string()) });
                row += 1;
            }
        }
    }
    out
}

enum ReadEvent {
    Stop,
    Mouse(u16, u16),
    Scroll(i16),
    Line(usize)
}

fn handle_event(buf: &TextBuffer, evt: Event) -> Vec<ReadEvent> {
    let mut out = Vec::new();
    match evt {
        Event::Key(Key::Char('q')) => out.push(ReadEvent::Stop),
        Event::Key(Key::Char('j')) => out.push(ReadEvent::Scroll(1)),
        Event::Key(Key::Char('k')) => out.push(ReadEvent::Scroll(-1)),
        Event::Key(Key::Char('g')) => out.push(ReadEvent::Line(1)),
        Event::Key(Key::Char('G')) => out.push(ReadEvent::Line(buf.text.len_lines())),

        Event::Key(Key::Char(c)) => {},
        //Key::Alt(c) => println!("^{}", c),
        //Key::Ctrl(c) => println!("*{}", c),
        Event::Key(Key::Esc) => out.push(ReadEvent::Stop),
        //Key::Left => println!("←"),
        //Key::Right => println!("→"),
        //Key::Up => println!("↑"),
        //Key::Down => println!("↓"),
        //Key::Backspace => println!("×"),
        Event::Mouse(me) => {
            match me {
                MouseEvent::Press(_, a, b) |
                MouseEvent::Release(a, b) |
                MouseEvent::Hold(a, b) => {
                    out.push(ReadEvent::Mouse(a, b));
                },
                _ => (),
            }
        }
        _ => {}
    };
    out
}

fn main() {
    let matches = App::new("editor")
        .version("0.1.0")
        .arg(Arg::with_name("d")
            .short("d")
            .help("Debug flag"))
        .arg(Arg::with_name("INPUT")
            .help("File to edit")
            .required(true)
            .index(1))
        .get_matches();

    if !termion::is_tty(&fs::File::create("/dev/stdout").unwrap()) {
        panic!("Not a tty");
    }

    // Get filepath from commandline
    let filepath = matches.value_of("INPUT").unwrap();
    let mut buf = TextBuffer::from_path(&filepath).unwrap();

    if matches.is_present("d") {
        for command in generate_commands(&buf) {
            println!("{:?}", command);
        }
    } else {
        // set unbuffered
        let stdin = stdin();
        let mut stdout = MouseTerminal::from(stdout().into_raw_mode().unwrap());
        reset();
        render(&buf, generate_commands(&buf));
        for c in stdin.events() {
            let evt = c.unwrap();
            for read_event in handle_event(&buf, evt) {
                match read_event {
                    ReadEvent::Stop => return,
                    ReadEvent::Scroll(dy) => {
                        let mut offset: i32 = buf.line_offset as i32;
                        offset += dy as i32;
                        if offset < 0 {
                            offset = 0;
                        } else if offset >= buf.text.len_lines() as i32 {
                            offset = buf.text.len_lines() as i32 - 1;
                        }
                        buf.line_offset = offset as usize;
                    }
                    ReadEvent::Line(line) => {
                        buf.line_offset = line - 1;
                    }

                    ReadEvent::Mouse(x, y) => {
                        buf.update_cursor(x as u32, y as u32);
                    }
                }
            }
            render(&buf, generate_commands(&buf));
        }
    }
}


