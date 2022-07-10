#![allow(dead_code)]
use super::*;
use crossbeam::channel;
use crossterm::cursor;
use crossterm::event;
use crossterm::event::{poll, Event};
use crossterm::execute;
use crossterm::style::Stylize;
use crossterm::terminal;
use crossterm::{queue, style, terminal::ClearType};
use editor_bindings::InputReader;
use editor_core::Command;
use log::*;
use std::convert::TryInto;
use std::io;
use std::io::{Stdout, Write};
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use termios::*;

lazy_static::lazy_static! {
    static ref IN_TERMINAL: AtomicBool = AtomicBool::new(false);
}

pub struct Terminal {
    ios: Termios,
    out: std::io::Stdout,
}

impl Default for Terminal {
    fn default() -> Self {
        let out = std::io::stdout();
        let ios = Termios::from_fd(out.as_raw_fd()).unwrap();
        Self { ios, out }
    }
}
impl Terminal {
    pub fn toggle(&mut self) {
        info!("toggle");
        match IN_TERMINAL.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |in_terminal| {
            Some(!in_terminal)
        }) {
            Ok(in_terminal) => {
                if in_terminal {
                    self.leave_raw_mode();
                } else {
                    self.enter_raw_mode();
                }
            }
            Err(e) => {
                error!("toggle: {:?}", e);
            }
        }
    }

    pub fn enter_raw_mode(&mut self) {
        info!("enter raw terminal");
        terminal::enable_raw_mode().unwrap();
        execute!(
            self.out,
            terminal::EnterAlternateScreen,
            terminal::Clear(terminal::ClearType::All),
            event::EnableMouseCapture,
            terminal::DisableLineWrap,
        )
        .unwrap();
    }

    pub fn leave_raw_mode(&mut self) {
        info!("leave terminal raw");
        execute!(
            self.out,
            event::DisableMouseCapture,
            terminal::EnableLineWrap,
            terminal::LeaveAlternateScreen,
        )
        .unwrap();
        terminal::disable_raw_mode().unwrap();
    }

    fn enable_signals(&mut self) {
        let mut ios = Termios::from_fd(self.out.as_raw_fd()).unwrap();
        ios.c_lflag &= !ISIG;
        match tcsetattr(self.out.as_raw_fd(), TCSAFLUSH, &ios) {
            Ok(x) => {
                info!("signal terminal success {:?}", x);
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {
                error!("retry terminal signal: {:?}", e);
                self.enter_attributes();
            }
            Err(e) => {
                error!("unable to update terminal: {:?}", e);
            }
        };
    }

    fn enter_attributes(&mut self) {
        // we need to do some termios magic
        // all of the rust terminal libraries disable signals and don't provide a way to catch them
        // properly.  We enter raw mode here, but leave signals to be caught and handled by the
        // application
        //
        // Lots of help here to understand what's going on:
        // https://viewsourcecode.org/snaptoken/kilo/02.enteringRawMode.html
        let mut ios = Termios::from_fd(self.out.as_raw_fd()).unwrap();
        let lflags0 = ios.c_lflag;

        ios.c_iflag &= !(
            BRKINT | // disable break condition signal
            INPCK |  // disable parity checking
            ISTRIP | // disable 8th bit stripping (just to be safe)
            ICRNL |  // disable carriage return translation
            IXON
            //disable software flow control
        );

        ios.c_cflag |= CS8; // character size set to 8bit

        ios.c_oflag &= !(
            OPOST
            // disable all output processing (carriage return and line feed translations)
        );

        ios.c_lflag &= !(
            //ISIG | // Terminal signals SIGTSTP
            IEXTEN | // Fix Ctrl-O in Macos, and disable Ctrl-V, for literal characters
            ICANON | // turn off canonical mode, so we read byte by byte, rather than line buffered
            ECHO
            // disable echo, causes characters to be echoed to the terminal
        );
        match tcsetattr(self.out.as_raw_fd(), TCSAFLUSH, &ios) {
            Ok(x) => {
                info!("enter terminal success {:?}", x);
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {
                error!("retry terminal enter: {:?}", e);
                self.enter_attributes();
            }
            Err(e) => {
                error!("unable to enter terminal: {:?}", e);
            }
        };
        let lflags1 = ios.c_lflag;
        info!("Enter Raw: {:?}", (lflags0, lflags1, ios));
    }

    fn leave_attributes(&mut self) {
        let mut ios = Termios::from_fd(self.out.as_raw_fd()).unwrap();
        let _lflags0 = ios.c_lflag;

        ios.c_iflag &= BRKINT | // disable break condition signal
            INPCK |  // disable parity checking
            ISTRIP | // disable 8th bit stripping (just to be safe)
            ICRNL |  // disable carriage return translation
            IXON; //disable software flow control

        ios.c_cflag |= CS8; // character size set to 8bit

        ios.c_oflag &= OPOST; // disable all output processing (carriage return and line feed translations)

        ios.c_lflag &= ISIG |
            IEXTEN | // Fix Ctrl-O in Macos, and disable Ctrl-V, for literal characters
            ICANON | // turn off canonical mode, so we read byte by byte, rather than line buffered
            ECHO; // disable echo, causes characters to be echoed to the terminal
        match tcsetattr(self.out.as_raw_fd(), TCSAFLUSH, &ios) {
            Ok(x) => {
                info!("leave terminal success {:?}", x);
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {
                error!("retry terminal leave: {:?}", e);
                self.leave_attributes();
            }
            Err(e) => {
                error!("unable to leave terminal: {:?}", e);
            }
        };
    }

    pub fn cleanup(&mut self) {
        self.leave_raw_mode();
    }

    pub fn render_reset(&mut self) {
        render_reset(&mut self.out)
    }

    pub fn render_commands(&mut self, commands: Vec<DrawCommand>) {
        render_commands(&mut self.out, commands)
    }
}

pub fn render_reset(out: &mut Stdout) {
    queue!(
        out,
        style::ResetColor,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )
    .unwrap();
    out.flush().unwrap();
}

pub fn render_commands(out: &mut Stdout, commands: Vec<DrawCommand>) {
    info!("C: {:?}", commands.len());
    if commands.len() == 0 {
        return;
    }

    queue!(out, cursor::Hide,).unwrap();
    for command in commands {
        handle_command(out, &command);
    }
    queue!(out, cursor::Show,).unwrap();
    out.flush().unwrap();
}

fn handle_command(out: &mut Stdout, command: &DrawCommand) {
    use DrawCommand::*;
    use LineFormatType::*;

    match command {
        SavePosition => {
            queue!(out, cursor::SavePosition).unwrap();
        }
        RestorePosition => {
            queue!(out, cursor::RestorePosition).unwrap();
        }
        Format(x, y, w, formats) => {
            debug!("F:{:?}", (x, y, w, formats));
            let s = format!("{:empty$}", " ", empty = w);
            queue!(
                out,
                cursor::MoveTo(*x as u16, *y as u16),
                style::Print(s),
                cursor::MoveTo(*x as u16, *y as u16),
            )
            .unwrap();
            for f in formats.iter() {
                let s = f.s.clone();
                match f.format {
                    Normal => queue!(out, style::Print(s)).unwrap(),
                    Highlight => queue!(out, style::Print(s.negative())).unwrap(),
                    Bold => queue!(out, style::Print(s.bold())).unwrap(),
                    Dim => queue!(out, style::Print(s.dim())).unwrap(),
                }
            }
        }

        DrawCommand::Status(row, s) => {
            queue!(
                out,
                cursor::MoveTo(0, *row),
                terminal::Clear(ClearType::CurrentLine),
                style::Print(s.clone().negative())
            )
            .unwrap();
        }

        DrawCommand::Row(x, y, s) => {
            queue!(
                out,
                cursor::MoveTo(*x, *y),
                terminal::Clear(ClearType::CurrentLine),
                style::Print(s),
            )
            .unwrap();
        }

        DrawCommand::Line(row, line, s) => {
            let fs;
            if *line > 0 {
                fs = format!("{:5} {}", line, s)
            } else {
                fs = format!("{:5} {}", " ", s)
            }

            queue!(
                out,
                cursor::MoveTo(0, *row),
                terminal::Clear(ClearType::CurrentLine),
                style::Print(fs)
            )
            .unwrap();
        }
        DrawCommand::Clear(x, y) => {
            queue!(
                out,
                cursor::MoveTo(*x as u16, *y as u16),
                terminal::Clear(ClearType::CurrentLine),
            )
            .unwrap();
        }
        DrawCommand::Cursor(a, b) => {
            debug!("Cursor: {:?}", (a, b));
            queue!(out, cursor::MoveTo(*a, *b),).unwrap();
        }
    }
}

#[derive(Debug)]
pub struct TokenError {}

pub fn event_to_command(event: Event) -> Result<Command, TokenError> {
    use crossterm::event::*;
    match event {
        Event::Resize(x, y) => Ok(Command::Resize(x, y)),
        Event::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: _,
        }) => match kind {
            MouseEventKind::ScrollUp => Ok(Command::Scroll(1)),
            MouseEventKind::ScrollDown => Ok(Command::Scroll(-1)),
            MouseEventKind::Moved => Ok(Command::Mouse(column, row)),
            _ => Err(TokenError {}),
        },
        _ => Err(TokenError {}),
    }
}

pub fn input_thread(
    reader: &mut InputReader,
    tx_background: channel::Sender<Command>,
    rx_background: channel::Receiver<Command>,
) {
    loop {
        match poll(std::time::Duration::from_millis(100)) {
            Ok(true) => {
                let event = crossterm::event::read().unwrap();
                info!("Event {:?}", event);

                let command: Result<Command, _> = event_to_command(event);

                // see if we got an immediate command
                match command {
                    Ok(Command::Quit) => {
                        info!("Command Quit");
                        reader.tx.send(Command::Quit).unwrap();
                        break;
                    }
                    Ok(c) => {
                        info!("Direct Command {:?}", c);
                        reader.tx.send(c).unwrap();
                    }
                    _ => (),
                }
                // parse user input
                match event.try_into() {
                    Ok(e) => {
                        reader.add(e);
                        if reader.is_quit() {
                            break;
                        }
                    }
                    Err(err) => {
                        info!("ERR: {:?}\r", (err));
                    }
                }
            }

            // behave like a background thread
            Ok(false) => match rx_background.try_recv() {
                Ok(Command::Quit) => {
                    info!("input quit");
                    tx_background.send(Command::Quit).unwrap();
                    break;
                }
                _ => (),
            },
            Err(err) => {
                info!("ERR: {:?}\r", (err));
            }
        }
    }
    info!("Input thread finished");
}
