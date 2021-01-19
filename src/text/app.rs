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
            }
            Command::Test => {
                self.test()
            }
            Command::MoveCursorX(dx) => {
                //self.move_cursor_x(self.char_current, dx);
            }
            Command::MoveCursorY(dy) => {
                //self.move_cursor_y(self.char_current, dy);
            }
            Command::ScrollPage(dy) => {
                //let xdy = self.view.vsy as f32 / dy as f32;
                //self.scroll(xdy as i32);
            }
            Command::Scroll(dy) => {
                //self.scroll(dy as i32);
            }

            Command::LineNav(x) => {
                //self.line_move(x);
            }

            // Goto a line
            Command::Line(line) => {
                //self.scroll_line(line);
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

                //let ViewSpec { x0, y0, sx, sy, ..} = self.view.spec;
                //let x1 = x0 + sx;
                //let y1 = y0 + sy;
                //if x >= x0  && x < sx && y >= y0 && y < y1 {
                    //let mut cx = x as usize - x0 as usize;
                    //let cy = y as usize - y0 as usize;
                    //let line = self.view.lines.get(cy).unwrap();
                    //match line.body {
                        //RowType::Line(_) => {
                            //let line_length = line.c1 - line.c0;
                            //if cx >= line_length {
                                //cx = line_length - 1;
                            //}
                            //let c = line.c0 + cx;
                            //info!("C: {:?}", (cx,cy, c, x, y, line_length, x1, y1, line, &self.view.spec));
                            //self.view.update_cursor(c);
                        //}
                        //_ => ()
                    //}
                    ////self.update_window(c);
                //}
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

pub fn app_debug(filepath: &str) {
    let mut fe = crate::frontend_debug::FrontendDebug::new();
    let mut buf = SmartBuffer::from_path(filepath).unwrap();
    let mut app = App::new(&mut buf, 20, 10);
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


