use log::*;
use termios::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::os::unix::io::AsRawFd;
use crossterm::{execute};
use crossterm::terminal;
use crossterm::event;
use crossterm::cursor;
use std::io;
use std::{io::{Write, Stdout}};
use crossterm::{
    queue, style,
    terminal::ClearType
};
use crossterm::style::Styler;
use super::*;

lazy_static::lazy_static! {
    static ref g_in_terminal: AtomicBool = AtomicBool::new(false);
}

pub struct Terminal {
    ios: Termios,
    out: std::io::Stdout,
}
impl Default for Terminal {
    fn default() -> Self {
        let mut out = std::io::stdout();
        let mut ios = Termios::from_fd(out.as_raw_fd()).unwrap();
        Self { ios, out }
    }
}
impl Terminal {
    pub fn toggle(&mut self) {
        info!("toggle");
        match g_in_terminal.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |in_terminal| Some(!in_terminal)) {
            Ok(in_terminal) => {
                if in_terminal {
                    self.leave_raw_mode();
                } else {
                    self.enter_raw_mode();
                }
            },
            Err(e) => {
                error!("toggle: {:?}", e);
            }
        }

    }

    pub fn enter_raw_mode(&mut self) {
        info!("enter raw terminal");
        //self.enable_signals();
        terminal::enable_raw_mode().unwrap();
        //self.enable_signals();
        //self.enter_attributes();
        execute!(self.out,
            //cursor::SavePosition,
            terminal::EnterAlternateScreen,
            terminal::Clear(terminal::ClearType::All),
            event::EnableMouseCapture,
            terminal::DisableLineWrap,
            ).unwrap();
        //self.out.flush().unwrap();
    }

    pub fn leave_raw_mode(&mut self) {
        info!("leave terminal raw");
        //leave_raw_mode/
        execute!(self.out,
            event::DisableMouseCapture,
            terminal::EnableLineWrap,
            terminal::LeaveAlternateScreen,
            //cursor::RestorePosition
            ).unwrap();
        terminal::disable_raw_mode().unwrap();
        //self.leave_attributes();
        //use std::io::Write;
        //self.out.flush().unwrap();
    }

    fn enable_signals(&mut self ) {
        let mut ios = Termios::from_fd(self.out.as_raw_fd()).unwrap();
        ios.c_lflag &= !ISIG;
        match tcsetattr(self.out.as_raw_fd(), TCSAFLUSH, &ios) {
            Ok(x) => {
                info!("signal terminal success {:?}", x);
            },
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
            IXON     //disable software flow control
        );

        ios.c_cflag |= CS8;  // character size set to 8bit

        ios.c_oflag &= !(
            OPOST // disable all output processing (carriage return and line feed translations)
        );

        ios.c_lflag &= !(
            //ISIG | // Terminal signals SIGTSTP
            IEXTEN | // Fix Ctrl-O in Macos, and disable Ctrl-V, for literal characters
            ICANON | // turn off canonical mode, so we read byte by byte, rather than line buffered
            ECHO     // disable echo, causes characters to be echoed to the terminal
        );
        match tcsetattr(self.out.as_raw_fd(), TCSAFLUSH, &ios) {
            Ok(x) => {
                info!("enter terminal success {:?}", x);
            },
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {
                error!("retry terminal enter: {:?}", e);
                self.enter_attributes();
            }
            Err(e) => {
                error!("unable to enter terminal: {:?}", e);
            }
        };
        let lflags1 = ios.c_lflag;
        info!("Enter Raw: {:?}", (lflags0, lflags1, ios) );
    }


    fn leave_attributes(&mut self) {
        let mut ios = Termios::from_fd(self.out.as_raw_fd()).unwrap();
        let lflags0 = ios.c_lflag;

        ios.c_iflag &= (
            BRKINT | // disable break condition signal
            INPCK |  // disable parity checking
            ISTRIP | // disable 8th bit stripping (just to be safe)
            ICRNL |  // disable carriage return translation
            IXON     //disable software flow control
        );

        ios.c_cflag |= CS8;  // character size set to 8bit

        ios.c_oflag &= (
            OPOST // disable all output processing (carriage return and line feed translations)
        );

        ios.c_lflag &= (
            ISIG |
            IEXTEN | // Fix Ctrl-O in Macos, and disable Ctrl-V, for literal characters
            ICANON | // turn off canonical mode, so we read byte by byte, rather than line buffered
            ECHO     // disable echo, causes characters to be echoed to the terminal
        );
        match tcsetattr(self.out.as_raw_fd(), TCSAFLUSH, &ios) {
            Ok(x) => {
                info!("leave terminal success {:?}", x);
            },
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
        //execute!(self.out, terminal::Clear(terminal::ClearType::All)).unwrap();
        //execute!(out, color::Fg(color::Reset), color::Bg(color::Reset)).unwrap();
        self.leave_raw_mode();
        //execute!(self.out, terminal::LeaveAlternateScreen).unwrap();
        //terminal::disable_raw_mode().unwrap();
    }

    pub fn render_reset(&mut self) {
        render_reset(&mut self.out)
    }
    pub fn render_flush(&mut self) {
        render_flush(&mut self.out)
    }

    pub fn render_commands(&mut self, commands: Vec<DrawCommand>) {
        render_commands(&mut self.out, commands)
    }
}

pub fn render_reset(out: &mut Stdout) {
    queue!(out,
        style::ResetColor,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0,0)
    ).unwrap();
    out.flush().unwrap();
}

pub fn render_commands(out: &mut Stdout, commands: Vec<DrawCommand>) {
    info!("C: {:?}", commands.len());
    if commands.len() == 0 {
        return;
    }

    queue!(out,
        cursor::Hide,
    ).unwrap();
    for command in commands {
        handle_command(out, &command);
    }
    queue!(out,
        cursor::Show,
    ).unwrap();
    out.flush().unwrap();
}

pub fn render_flush(out: &mut Stdout) {
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
            let s = format!("{:empty$}", " ", empty=w);
            queue!(out,
                cursor::MoveTo(*x as u16, *y as u16),
                style::Print(s),
                cursor::MoveTo(*x as u16, *y as u16),
            ).unwrap();
            for f in formats.iter() {
                let s = f.1.clone();
                match f.0 {
                    Normal => queue!(out, style::Print(s)).unwrap(),
                    Highlight => queue!(out, style::Print(s.negative())).unwrap(),
                    Dim => queue!(out, style::Print(s.dim())).unwrap(),
                }
            }
        }

        DrawCommand::Status(row, s) => {
            queue!(out,
                cursor::MoveTo(0, *row),
                terminal::Clear(ClearType::CurrentLine),
                style::Print(s.clone().negative())
            ).unwrap();
        },

        DrawCommand::Row(x, y, s) => {
            queue!(out,
                cursor::MoveTo(*x, *y),
                terminal::Clear(ClearType::CurrentLine),
                style::Print(s),
            ).unwrap();
        }

        DrawCommand::Line(row, line, s) => {
            let fs;
            if *line > 0 {
                fs = format!("{:5} {}", line, s)
            } else {
                fs = format!("{:5} {}", " ", s)
            }

            queue!(out,
                cursor::MoveTo(0, *row),
                terminal::Clear(ClearType::CurrentLine),
                style::Print(fs)
            ).unwrap();
        },
        DrawCommand::Clear(x, y) => {
            queue!(out,
                cursor::MoveTo(*x as u16, *y as u16),
                terminal::Clear(ClearType::CurrentLine),
            ).unwrap();
        }
        DrawCommand::Cursor(a, b) => {
            debug!("Cursor: {:?}", (a, b));
            queue!(out,
                cursor::MoveTo(*a, *b),
            ).unwrap();
        }
    }
}


