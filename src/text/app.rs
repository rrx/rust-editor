use log::*;
use crate::ism::FrontendTrait;
use std::convert::TryInto;
use super::*;

#[derive(Debug)]
pub struct App<'a> {
    view: BufferView<'a>
}

impl<'a> App<'a> {
    pub fn new(buf: &'a mut SmartBuffer<'a>, x: u16, y: u16) -> Self {
        let spec = ViewSpec::new(x, y, 0, 0);
        let mut s = Self {
            view: BufferView::new(buf, Mode::Normal, spec)
        };
        s.resize(x, y, 0, 0);
        s
    }

    fn resize(&mut self, w: u16, h: u16, origin_x: u16, origin_y: u16) {
        self.view.resize(w, h, origin_x, origin_y);
        self.view.update_lines();
        self.view.refresh();
    }

    fn test(&mut self) {
        self.view.spec.rm += 1;
        self.view.spec.calc();
        let ViewSpec {w, h, origin_x: x, origin_y: y, ..} = self.view.spec;
        self.resize(w, h - 1, x, y + 1);
        info!("T: {:?}", (self.view.spec));
    }

    fn command(&mut self, command: Command) {
        info!("Command: {:?}", command);
        match command {
            Command::Mode(m) => {
                self.view.mode = m;
                self.view.update_cursor(self.view.port.char_current);

            }
            Command::Test => {
                self.test()
            }
            Command::MoveCursorX(dx) => {
                let (c, x_hint) = self.view.move_cursor_x(self.view.port.char_current, dx);
                self.view.cursor.set_x_hint(x_hint as u16);
                self.view.update_cursor(c);
            }
            Command::MoveCursorY(dy) => {
                let c = self.view.move_cursor_y(self.view.port.char_current, dy);
                self.view.update_cursor(c);
            }
            Command::ScrollPage(dy) => {
                let xdy = self.view.spec.sy as f32 / dy as f32;
                self.view.scroll(xdy as i32);
            }
            Command::Scroll(dy) => {
                self.view.scroll(dy as i32);
            }

            Command::LineNav(x) => {
                match self.view.char_to_wrap(self.view.port.char_current) {
                    Some(w) => {
                        let c = self.view.line_move(x);
                        let dx = c - w.c0;
                        self.view.cursor.set_x_hint(dx as u16);
                        self.view.update_cursor(c);
                    }
                    _ => ()
                }

            }

            // Goto a line
            Command::Line(line) => {
                let c = self.view.jump_to_line(line);
                self.view.update_cursor(c);
            }

            Command::Resize(x, y) => {
                self.resize(x, y, 0, 0);
            }

            Command::Mouse(x, y) => {
                match self.view.char_from_cursor(x, y) {
                    Some(c) => {
                        self.view.update_cursor(c);
                    }
                    _ => ()
                }
            }
            _ => self.view.command(command)
        }
    }

    pub fn process(&mut self, fe: &mut dyn FrontendTrait) {
        let mut q = Vec::new();
        fe.reset();
        fe.render(self.view.render());
        loop {
            let event = crossterm::event::read().unwrap();

            // see if we got a command
            match event.try_into() {
                Ok(Command::Quit) => {
                    info!("Quit");
                    return;
                }
                Ok(c) => {
                    self.command(c);
                    fe.render(self.view.render());
                    continue;
                }
                _ => ()
            }

            // run parse otherwise
            match event.try_into() {
                Ok(e) => {
                    q.push(e);
                    let result = self.view.mode.command()(q.as_slice());
                    match result {
                        Ok((_, Command::Quit)) => {
                            info!("Quit");
                            return;
                        }
                        Ok((_, x)) => {
                            info!("[{:?}] Ok: {:?}\r", &self.view.mode, (&q, &x));
                            q.clear();
                            self.command(x);
                            fe.render(self.view.render());
                        }
                        Err(nom::Err::Incomplete(_)) => {
                            info!("Incomplete: {:?}\r", (q));
                        }
                        Err(e) => {
                            info!("Error: {:?}\r", (e, &q));
                            q.clear();
                        }
                    }
                }
                Err(err) => {
                    info!("ERR: {:?}\r", (err));
                }
            }
        }
    }

}

pub fn debug(filepath: &str) {
    let mut fe = crate::frontend_debug::FrontendDebug::new();
    let mut buf = SmartBuffer::from_path(filepath).unwrap();
    let mut app = App::new(&mut buf, 20, 10);
    log::info!("X");
    app.process(&mut fe);
}

pub fn raw(filepath: &str) {
    use crossterm::*;
    use crossterm::terminal::*;
    use crossterm::event::*;
    // set initial size
    let mut fe = crate::frontend_crossterm::FrontendCrossterm::new();
    let (sx, sy) = terminal::size().unwrap();
    let mut buf = SmartBuffer::from_path(filepath).unwrap();
    let mut app = App::new(&mut buf, sx, sy);
    let mut out = std::io::stdout();
    enable_raw_mode().unwrap();
    execute!(out, EnableMouseCapture).unwrap();
    app.process(&mut fe);
    execute!(out, DisableMouseCapture).unwrap();
    disable_raw_mode().unwrap();
}

